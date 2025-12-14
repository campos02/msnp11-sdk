#[tokio::test]
async fn tabs() {
    let client = msnp11_sdk::client::Client::new(&"127.0.0.1".to_string(), 1863)
        .await
        .unwrap();

    let tabs = client
        .get_tabs("http://localhost:3000/Config/MsgrConfig.asmx")
        .await
        .unwrap();

    assert_eq!(tabs.len(), 3);
    assert_eq!(tabs[0].name, "Wiby");
    assert_eq!(tabs[1].name, "FrogFind!");
    assert_eq!(tabs[2].name, "TheOldNet");
}
