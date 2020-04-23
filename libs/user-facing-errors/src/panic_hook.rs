pub fn set_panic_hook() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic| {
        let err = crate::Error::new_in_panic_hook(&panic);

        match serde_json::to_writer(std::io::stderr(), &err) {
            Ok(_) => eprintln!(),
            Err(err) => {
                tracing::error!("Failed to write JSON error to stderr: {}", err);
                original_hook(panic)
            }
        }

        std::process::exit(255)
    }));
}
