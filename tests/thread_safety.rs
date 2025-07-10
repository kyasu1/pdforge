use pdforge::schemas::Error;

// This test verifies that our Error type implements Send + Sync
// If it doesn't, this test will fail to compile
#[test]
fn error_is_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    
    assert_send::<Error>();
    assert_sync::<Error>();
}

// This also tests that we can send the error across threads
#[test]
fn error_can_be_sent_across_threads() {
    let error = Error::Whatever {
        message: "Test error".to_string(),
        source: None,
    };
    
    std::thread::spawn(move || {
        println!("Error in thread: {:?}", error);
    }).join().unwrap();
}