use mongodb::Client;

struct MongoTestConnector {
    client: Client,
}

fn connector() -> MongoTestConnector {}

#[test]
fn hi() {
    panic!("nope")
}
