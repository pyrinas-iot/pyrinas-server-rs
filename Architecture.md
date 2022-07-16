# Architecture

Here's some brief information about how Pyrias currently works. As things develop this document will get updated accordingly.

## Broker & message passing.

The broker is *the* center point of Pyrinas Server. It allows events that are generated anywhere in the application to be routed to where they need to go. 

Currently there are a few different types of events. As of this writing here they are:

```rust 
pub enum Event {
    NewRunner { name: String, sender: Sender<Event> },
    OtaDeletePackage(OtaUpdate),
    OtaNewPackage(OtaUpdate),
    OtaRequest { uid: String, msg: OtaRequest },
    OtaResponse(OtaUpdate),
    ApplicationManagementRequest(ManagementData), // Message sent for configuration of application
    ApplicationManagementResponse(ManagementData),// Reponse from application management portion of the app
    ApplicationRequest(ApplicationData),          // Request/event from a device
    ApplicationResponse(ApplicationData),         // Reponse from other parts of the server
    InfluxDataSave(WriteQuery),                   // Takes a pre-prepared query and executes it
    InfluxDataRequest(ReadQuery),                 // Takes a pre-prepared query to *read* the database
    InfluxDataResponse,                           // Is the response to InfluxDataRequest
}
```

Some of the most used events will be the `ApplicationRequest` and `ApplicationResponse` messages. These are generated when you send messages through MQTT to the server using the `app` topic. 

For example, in the topic `<uid>/app/p/data`, the server derives the uid, that it's an application message and that it's being published from a device with the target "data". All of this is sorted out in `mqtt.rs` in the `mqtt_run` function before it get's routed to the broker and then onto the application side of your code.

Currently the broker is implemented using unbounded `flume` channels. Every broker "client" needs to register using `Event::NewRunner` before beginning work. Here's an example:

```rust
// Get the sender/reciever associated with this particular task
let (sender, reciever) = unbounded::<Event>();

// Register this task
broker_sender
    .send_async(Event::NewRunner {
        name: "mqtt".to_string(),
        sender: sender.clone(),
    })
    .await
    .unwrap();
```

That way messages *from* the broker are recieved using `reciever`. Messages that need to go *to* the broker are sent using `broker_sender: Sender<Event>` that is usually passed into the init/run function for the broker client:

```rust
pub async fn run(tx: &mut AsyncLinkTx, broker_sender: Sender<Event>) {
```

### Function Specific Requests

As you can tell by the naming conventions in `Event` other events are function specific. One's with `Ota` are OTA update specific. `Influx` are data logging specific and so on. This is the "built-in" functionality of the server where you don't have to do anything for it to work besides enter the correct configuration information in `config.toml`.

## Mqtt

- [ ] Currently uses a modified version of `rumqttd` to serve secure clients using `native_tls`
- [ ] Configuration via `config.toml` 

## InfluxDB data collection

- [ ] Predefine the struct format for influx data
- [ ] Flaten structs where needed
- [ ] Once that's done you can send the raw query and be done with it.

## Administration

- [ ] Works over websockets using `ManagementData` 