//! CC1101 packet receiver for the WH24 and a 26 MHz crystal.
//!
//! Register definitions and timing: TI SWRS061I §§10, 12–15, 19, 20, 29.
//! https://www.ti.com/lit/ds/symlink/cc1101.pdf

use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_hal::{
    Blocking,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{GPIO0, GPIO1, GPIO18, GPIO19, GPIO20, SPI2},
    spi::{
        Mode,
        master::{Config, Spi},
    },
    time::Rate,
};

const SRES: u8 = 0x30;
const SRX: u8 = 0x34;
const SIDLE: u8 = 0x36;
const SFRX: u8 = 0x3a;
const PARTNUM: u8 = 0x30;
const VERSION: u8 = 0x31;
const MARCSTATE: u8 = 0x35;
const RXBYTES: u8 = 0x3b;
const RX: u8 = 0x0d;

// Calculated from the datasheet equations for f_XOSC = 26 MHz:
// FREQ=0x21656a -> 868299865 Hz; DRATE_E=9/M=92 -> 17257.7 baud;
// DEVIATION_E=4/M=3 -> 34912.1 Hz; CHANBW_E=2/M=0 -> 203125 Hz.
const PROFILE: &[(u8, u8)] = &[
    (0x00, 0x2e), // IOCFG2: unused, high impedance
    (0x01, 0x2e), // IOCFG1: unused except SPI SO
    (0x02, 0x06), // IOCFG0: sync asserted, packet end deasserted
    (0x03, 0x47), // FIFOTHR: ADC retention for bandwidth below 325 kHz
    (0x04, 0x2d), // SYNC1
    (0x05, 0xd4), // SYNC0
    (0x06, 17),   // PKTLEN: payload, excluding appended status
    (0x07, 0x44), // PKTCTRL1: PQT=2, append status, no address/CRC autoflush
    (0x08, 0x00), // PKTCTRL0: fixed length, no whitening/hardware CRC
    (0x0a, 0x00), // CHANNR
    (0x0b, 0x06), // FSCTRL1: IF 152.34 kHz
    (0x0c, 0x00), // FSCTRL0: no frequency offset
    (0x0d, 0x21), // FREQ2
    (0x0e, 0x65), // FREQ1
    (0x0f, 0x6a), // FREQ0
    (0x10, 0x89), // MDMCFG4: bandwidth + data rate exponent
    (0x11, 0x5c), // MDMCFG3: data rate mantissa
    (0x12, 0x02), // MDMCFG2: NRZ 2-FSK, strict 16/16 sync
    (0x13, 0x22), // MDMCFG1: FEC off (TX preamble unused)
    (0x14, 0xf8), // MDMCFG0: spacing unused with channel zero
    (0x15, 0x43), // DEVIATN
    (0x16, 0x07), // MCSM2: no RX timeout
    (0x17, 0x0c), // MCSM1: stay in RX after packet
    (0x18, 0x18), // MCSM0: calibrate IDLE -> RX
    (0x19, 0x16), // FOCCFG: carrier-gated tracking, limit BW/4
    (0x1a, 0x6c), // BSCFG: default bit synchronization
    (0x1b, 0x43), // AGCCTRL2: full LNA gain, target 33 dB
    (0x1c, 0x40), // AGCCTRL1
    (0x1d, 0x91), // AGCCTRL0
    (0x21, 0x56), // FREND1: RX front end
    (0x23, 0xe9), // FSCAL3: enable charge pump calibration
    (0x24, 0x2a), // FSCAL2: high-band VCO
    (0x25, 0x00), // FSCAL1: replaced by calibration
    (0x26, 0x1f), // FSCAL0: calibration control
    (0x2c, 0x81), // TEST2: bandwidth below 325 kHz
    (0x2d, 0x35), // TEST1: bandwidth below 325 kHz
    (0x2e, 0x09), // TEST0: use the high-band VCO selected by FSCAL2
];

#[derive(Debug)]
pub enum Error {
    Spi,
    NotReady,
    Identity {
        part: u8,
        version: u8,
    },
    Configuration {
        address: u8,
        expected: u8,
        actual: u8,
    },
    State(u8),
    Overflow,
    UnstableFifo,
    PacketTimeout,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Identity { part, version } => write!(
                f,
                "unexpected identity: part=0x{part:02x} version=0x{version:02x}"
            ),
            Self::Configuration {
                address,
                expected,
                actual,
            } => write!(
                f,
                "register 0x{address:02x}: expected 0x{expected:02x}, read 0x{actual:02x}"
            ),
            Self::State(state) => write!(f, "unexpected MARCSTATE=0x{state:02x}"),
            Self::NotReady => {
                f.write_str("SO/MISO readiness timeout (check CC1101 power and SPI wiring)")
            }
            Self::Spi => f.write_str("SPI transfer failed"),
            Self::Overflow => f.write_str("RX FIFO overflow"),
            Self::UnstableFifo => f.write_str("RX FIFO count did not stabilize"),
            Self::PacketTimeout => f.write_str("GDO0 packet completion timeout"),
        }
    }
}

pub struct Cc1101 {
    spi: Spi<'static, Blocking>,
    miso: Input<'static>,
    cs: Output<'static>,
    gdo0: Input<'static>,
}

impl Cc1101 {
    pub fn new(
        spi: SPI2<'static>,
        mosi: GPIO18<'static>,
        miso: GPIO20<'static>,
        sck: GPIO19<'static>,
        cs: GPIO0<'static>,
        gdo0: GPIO1<'static>,
    ) -> Self {
        // A disconnected SO should time out, not masquerade as a ready radio.
        let miso = Input::new(miso, InputConfig::default().with_pull(Pull::Up));
        let cs = Output::new(cs, Level::High, OutputConfig::default());
        let spi = Spi::new(
            spi,
            Config::default()
                .with_frequency(Rate::from_mhz(1))
                .with_mode(Mode::_0),
        )
        .expect("valid SPI configuration")
        .with_mosi(mosi)
        .with_miso(miso.peripheral_input())
        .with_sck(sck);
        Self {
            spi,
            miso,
            cs,
            gdo0: Input::new(gdo0, InputConfig::default().with_pull(Pull::Down)),
        }
    }

    pub async fn initialize(&mut self) -> Result<u8, Error> {
        self.reset().await?;
        let part = self.status(PARTNUM).await?;
        let version = self.status(VERSION).await?;
        if part != 0 || version == 0 || version == 0xff {
            return Err(Error::Identity { part, version });
        }
        for &(address, value) in PROFILE {
            self.transfer(&mut [address, value]).await?;
            let actual = self.register(address).await?;
            if actual != value {
                return Err(Error::Configuration {
                    address,
                    expected: value,
                    actual,
                });
            }
        }
        self.strobe(SIDLE).await?;
        self.wait_state(0x01).await?;
        self.strobe(SFRX).await?;
        self.strobe(SRX).await?;
        self.wait_state(RX).await?;
        Ok(version)
    }

    async fn reset(&mut self) -> Result<(), Error> {
        self.cs.set_high();
        Timer::after_millis(1).await;
        self.cs.set_low();
        Timer::after_millis(1).await;
        self.cs.set_high();
        Timer::after_millis(1).await;
        self.select().await?;
        let reset = self.spi.transfer(&mut [SRES]).map_err(|_| Error::Spi);
        // Keep CS asserted until reset completes, including its oscillator delay.
        Timer::after_millis(1).await;
        let ready = self.wait_ready().await;
        self.cs.set_high();
        reset?;
        ready?;

        Ok(())
    }

    /// Wait for GDO0 packet completion, with a periodic health/FIFO check to
    /// recover missed edges and keep the caller's silence deadlines advancing.
    pub async fn receive(&mut self) -> Result<Option<[u8; 19]>, Error> {
        if let Some(packet) = self.read_packet().await? {
            return Ok(Some(packet));
        }
        let _ = with_timeout(Duration::from_secs(1), self.gdo0.wait_for_falling_edge()).await;
        // A burst may straddle the health timer. Give its 8 ms payload time to
        // finish; a stuck-high GDO0 must not suspend the task indefinitely.
        with_timeout(Duration::from_millis(30), self.gdo0.wait_for_low())
            .await
            .map_err(|_| Error::PacketTimeout)?;
        self.read_packet().await
    }

    async fn read_packet(&mut self) -> Result<Option<[u8; 19]>, Error> {
        let mut count = self.status(RXBYTES).await?;
        // RXBYTES can be sampled during a radio clock update (TI errata).
        // Require two equal reads and bound the number of attempts.
        for _ in 0..8 {
            let next = self.status(RXBYTES).await?;
            if next != count {
                count = next;
                continue;
            }
            if count & 0x80 != 0 {
                return Err(Error::Overflow);
            }
            if count >= 19 {
                let mut bytes = [0_u8; 20];
                bytes[0] = 0xff; // FIFO burst read: 0x3f | READ | BURST
                self.transfer(&mut bytes).await?;
                let mut packet = [0_u8; 19];
                packet.copy_from_slice(&bytes[1..]);
                return Ok(Some(packet));
            }
            // RX_END/RX_RST are brief transitions in continuous receive.
            let state = self.status(MARCSTATE).await? & 0x1f;
            if !matches!(state, RX | 0x0e | 0x0f) {
                return Err(Error::State(state));
            }
            return Ok(None);
        }
        Err(Error::UnstableFifo)
    }

    async fn wait_ready(&self) -> Result<(), Error> {
        let deadline = Instant::now() + Duration::from_millis(10);
        while self.miso.is_high() {
            if Instant::now() >= deadline {
                return Err(Error::NotReady);
            }
            Timer::after_micros(100).await;
        }
        Ok(())
    }

    async fn select(&mut self) -> Result<(), Error> {
        self.cs.set_low();
        if let Err(error) = self.wait_ready().await {
            self.cs.set_high();
            return Err(error);
        }
        Ok(())
    }

    async fn transfer(&mut self, bytes: &mut [u8]) -> Result<(), Error> {
        self.select().await?;
        // At most 20 bytes at 1 MHz; no external clock or readiness wait here.
        let result = self.spi.transfer(bytes).map_err(|_| Error::Spi);
        self.cs.set_high();
        result
    }

    async fn strobe(&mut self, command: u8) -> Result<(), Error> {
        self.transfer(&mut [command]).await
    }

    async fn register(&mut self, address: u8) -> Result<u8, Error> {
        let mut bytes = [address | 0x80, 0];
        self.transfer(&mut bytes).await?;
        Ok(bytes[1])
    }

    async fn status(&mut self, address: u8) -> Result<u8, Error> {
        // Status registers require BURST even for a single byte.
        let mut bytes = [address | 0xc0, 0];
        self.transfer(&mut bytes).await?;
        Ok(bytes[1])
    }

    async fn wait_state(&mut self, expected: u8) -> Result<(), Error> {
        let deadline = Instant::now() + Duration::from_millis(20);
        loop {
            let state = self.status(MARCSTATE).await? & 0x1f;
            if state == expected {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::State(state));
            }
            Timer::after_millis(1).await;
        }
    }
}
