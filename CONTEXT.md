# Climate Data Gateway

The gateway collects indoor readings and readings transmitted by a weather station for reporting.

## Language

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
