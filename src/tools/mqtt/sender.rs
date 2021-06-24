use super::*;
use crate::init::info::Information;
use crate::tools::Auth;
use anyhow::Context;
use std::time::Duration;
use uuid::Uuid;

pub struct MqttSender {
    client: paho_mqtt::AsyncClient,
}

impl MqttSender {
    pub async fn new(info: &Information, auth: Auth, version: MqttVersion) -> anyhow::Result<Self> {
        let client_id = Uuid::new_v4().to_string();

        let uri = format!("ssl://{}:{}", info.mqtt.host, info.mqtt.port);

        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(uri)
            .client_id(client_id)
            .persistence(paho_mqtt::PersistenceType::None);

        let create_opts = match version {
            MqttVersion::V3_1_1 => create_opts.mqtt_version(paho_mqtt::MQTT_VERSION_3_1_1),
            MqttVersion::V5(_) => create_opts.mqtt_version(paho_mqtt::MQTT_VERSION_5),
        };

        let client = paho_mqtt::AsyncClient::new(create_opts.finalize())
            .context("Failed to create client")?;

        let mut ssl_opts = paho_mqtt::SslOptionsBuilder::new();
        ssl_opts.enable_server_cert_auth(false).verify(false);

        let mut conn_opts = paho_mqtt::ConnectOptionsBuilder::new();

        match auth {
            Auth::None => {}
            Auth::UsernamePassword(username, password) => {
                conn_opts.user_name(username).password(password);
            }
            Auth::X509Certificate(cert) => {
                unimplemented!("X.509 client certificates are not implemented in MQTT tests yet");
            }
        }

        conn_opts
            .ssl_options(ssl_opts.finalize())
            .keep_alive_interval(Duration::from_secs(30))
            .automatic_reconnect(Duration::from_millis(100), Duration::from_secs(5));

        version.apply(&mut conn_opts);

        client
            .connect(conn_opts.finalize())
            .await
            .context("Failed to connect")?;

        Ok(Self { client })
    }

    pub async fn send(
        &self,
        channel: String,
        qos: MqttQoS,
        content_type: String,
        payload: Option<Vec<u8>>,
    ) -> anyhow::Result<()> {
        let mut props = paho_mqtt::Properties::new();
        props.push_string(paho_mqtt::PropertyCode::ContentType, &content_type)?;

        let msg = paho_mqtt::MessageBuilder::new()
            .topic(format!("{}", channel))
            .payload(payload.unwrap_or_default())
            .qos(qos.into())
            .properties(props);

        Ok(self.client.try_publish(msg.finalize())?.await?)
    }
}