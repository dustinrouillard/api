use envconfig::Envconfig;
use lapin::{
  options::{BasicPublishOptions, QueueDeclareOptions},
  types::FieldTable,
  BasicProperties, Channel, Connection, ConnectionProperties, Queue,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
  config::Config,
  structs::{boosted::BoostedRideUpdate, spotify::CurrentPlaying},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RabbitEvent {
  SpotifyUpdate,
  BoostedUpdate,
}

#[derive(Clone)]
pub struct RabbitManager {
  pub channel: Channel,
  pub queue: Queue,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RabbitEventsData<Data = Value> {
  t: RabbitEvent,
  d: Data,
}

impl RabbitManager {
  pub async fn new() -> RabbitManager {
    let config = Config::init_from_env().unwrap();
    let connection = Connection::connect(
      &config.rabbit_dsn,
      ConnectionProperties::default(),
    )
    .await
    .unwrap();

    let channel = connection.create_channel().await.unwrap();

    let queue = channel
      .queue_declare(
        &config.rabbit_queue,
        QueueDeclareOptions {
          durable: true,
          ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
      )
      .await
      .unwrap();

    tracing::info!("Connected to rabbitmq");

    Self { channel, queue }
  }

  pub async fn publish_ride_state(&mut self, data: &BoostedRideUpdate) {
    let message = RabbitEventsData {
      t: RabbitEvent::BoostedUpdate,
      d: data.to_owned(),
    };
    let json = serde_json::to_string(&message).unwrap();

    self
      .channel
      .basic_publish(
        "",
        "dstn-gateway-ingest",
        BasicPublishOptions::default(),
        json.as_bytes(),
        BasicProperties::default(),
      )
      .await
      .unwrap();
  }

  pub async fn publish_spotify_current(&mut self, data: &CurrentPlaying) {
    let message = RabbitEventsData {
      t: RabbitEvent::SpotifyUpdate,
      d: data.to_owned(),
    };
    let json = serde_json::to_string(&message).unwrap();

    self
      .channel
      .basic_publish(
        "",
        "dstn-gateway-ingest",
        BasicPublishOptions::default(),
        json.as_bytes(),
        BasicProperties::default(),
      )
      .await
      .unwrap();
  }

  pub async fn publish_spotify_not_playing(&mut self) {
    let json =
      json!({"t": RabbitEvent::SpotifyUpdate, "d": {"playing": false}});

    self
      .channel
      .basic_publish(
        "",
        "dstn-gateway-ingest",
        BasicPublishOptions::default(),
        json.to_string().as_bytes(),
        BasicProperties::default(),
      )
      .await
      .unwrap();
  }
}
