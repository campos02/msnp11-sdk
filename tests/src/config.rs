#[tokio::test]
async fn config() {
    let client = msnp11_sdk::client::Client::new(&"127.0.0.1".to_string(), 1863)
        .await
        .unwrap();

    let config = client
        .get_config("http://localhost:3000/Config/MsgrConfig.asmx")
        .await
        .unwrap();

    let tabs = config.tabs;
    let msn_today_url = config.msn_today_url;

    assert_eq!(tabs.len(), 3);
    assert_eq!(tabs[0].name, "Wiby");
    assert_eq!(tabs[1].name, "FrogFind!");
    assert_eq!(tabs[2].name, "TheOldNet");
    assert_eq!(msn_today_url, "http://today.msgrsvcs.ctsrv.gay/start?msn=1");
}
