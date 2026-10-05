//! Internal macros shared across GUI feature modules.

/// Build an `iced::Task<Message>` that runs blocking work on a background
/// thread via a `futures` oneshot channel (keeps the UI executor free).
///
/// `$msg` is the `Message` constructor given to `Task::perform`; `$body` is an
/// expression evaluated on the spawned thread whose value is passed to it.
macro_rules! blocking_task {
    ($msg:expr, $body:expr) => {
        iced::Task::perform(
            async move {
                let (tx, rx) = futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = tx.send($body);
                });
                rx.await.map_err(|_| "Task cancelled".to_string())
            },
            $msg,
        )
    };
}
