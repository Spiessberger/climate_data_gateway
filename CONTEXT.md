# Climate Data Gateway

The gateway collects local climate measurements and readings transmitted by a weather station for reporting.

## Language

**Climate reading**:
A paired measurement of temperature and relative humidity from the same sensor measurement.
_Avoid_: Sensor value (ambiguous about which quantity it contains)

**Weather station**:
The outdoor Fine Offset WH24 sensor array that transmits weather measurements to the gateway.
_Avoid_: Temperature sensor (ambiguous with the local climate sensor)

**Weather reading**:
The measurements and sensor status carried by one accepted weather station transmission. A quantity marked unavailable by the station is missing from that reading.
_Avoid_: Climate reading (specifically the local paired temperature and humidity measurement)

**Station ID**:
The identifier transmitted by a weather station, which can change after its batteries are replaced. It identifies the source of a weather reading, rather than a permanent physical device identity.

**Selected station**:
The weather station whose readings the gateway currently accepts, identified by its station ID. Selection begins with the first valid transmission and is released after consecutive missed transmissions.
