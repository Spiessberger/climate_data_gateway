# Climate Data Gateway

The gateway collects indoor readings and readings transmitted by a weather station for reporting.
The climate data service receives climate readings and gateway logs, retains history, and serves applications that display live and historical data.

## Language

**Climate data service**:
The application that receives gateway readings and logs, preserves historical data, and provides live readings and historical queries to clients.
_Avoid_: Collector (describes only its acquisition responsibility), Pi application (the role is independent of its host)

**Climate reading**:
An indoor reading or a weather reading, including the source and sensor status available with that reading.

**Live reading**:
The latest valid climate reading received for the indoor or weather stream, available to clients regardless of whether it has been stored.

**Stored reading**:
A climate reading successfully preserved by the climate data service and available as historical data. A live reading is not necessarily a stored reading.

**Reception time**:
The time the climate data service receives a climate reading. It does not assert when the sensor measured the reported quantities.

**Gateway heartbeat**:
A periodic notification from the gateway that allows the climate data service to detect loss of communication independently of climate readings.

**Indoor sensor**:
The local sensor that measures indoor temperature and relative humidity.

**Indoor reading**:
A paired measurement of temperature and relative humidity from the same indoor sensor measurement.
_Avoid_: Sensor value (ambiguous about which quantity it contains)

**Weather station**:
The outdoor Fine Offset WH24 sensor array that transmits weather measurements to the gateway.
_Avoid_: Temperature sensor (ambiguous with the indoor sensor)

**Weather reading**:
The measurements and sensor status carried by one accepted weather station transmission. A quantity marked unavailable by the station is missing from that reading.
_Avoid_: Indoor reading (specifically the indoor paired temperature and humidity measurement)

**Station ID**:
The identifier transmitted by a weather station, which can change after its batteries are replaced. It identifies the source of a weather reading, rather than a permanent physical device identity.

**Selected station**:
The weather station whose readings the gateway currently accepts, identified by its station ID. Selection begins with the first valid transmission and is released after consecutive missed transmissions.
