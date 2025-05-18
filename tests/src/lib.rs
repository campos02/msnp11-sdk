use env_logger::Env;
use msnp11_sdk::client::Client;
use msnp11_sdk::event::Event;
use msnp11_sdk::models::personal_message::PersonalMessage;
use msnp11_sdk::models::presence::Presence;

#[tokio::test]
async fn login() {
    env_logger::Builder::from_env(Env::default().default_filter_or("trace")).init();

    let mut client = Client::new("127.0.0.1".to_string(), "1863".to_string())
        .await
        .unwrap();

    let result: Event = match client
        .login(
            "testing@example.com".to_string(),
            "123456".to_string(),
            "http://localhost:3000/rdr/pprdr.asp".to_string(),
        )
        .await
    {
        Ok(Event::RedirectedTo { server, port }) => {
            client = Client::new(server, port).await.unwrap();
            client
                .login(
                    "testing@example.com".to_string(),
                    "123456".to_string(),
                    "http://localhost:3000/rdr/pprdr.asp".to_string(),
                )
                .await
                .unwrap()
        }

        Ok(Event::Authenticated) => Event::Authenticated,
        Err(err) => panic!("Login error: {err}"),
        _ => Event::Disconnected,
    };

    assert_eq!(result, Event::Authenticated);

    client
        .set_presence(&Presence::new("NLN".to_string(), None))
        .await
        .unwrap();

    client
        .set_personal_message(&PersonalMessage {
            psm: "".to_string(),
            current_media: "".to_string(),
        })
        .await
        .unwrap();
}
