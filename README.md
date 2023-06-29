# vatsim-exporter
This is a Prometheus exporter for the VATSIM data feed. It includes metrics of online controllers and pilots.

## Available Metrics

 * `vatsim_airport_arrivals_current{icao,state}`
 * `vatsim_airport_departures_current{icao,state}`
 * `vatsim_controller_online_seconds_count{callsign,cid,name,facility}`
 * `vatsim_pilot_altitude{callsign,cid,name}`
 * `vatsim_pilot_groundspeed{callsign,cid,name}`
 * `vatsim_pilot_heading{callsign,cid,name}`
 * `vatsim_pilot_latitude{callsign,cid,name}`
 * `vatsim_pilot_longitude{callsign,cid,name}`
