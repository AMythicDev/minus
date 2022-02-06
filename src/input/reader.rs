#![allow(dead_code)]
use super::InputEvent;
use crate::{events::Event, MinusError, PagerState};
use crossbeam_channel::{Sender, TrySendError};
#[cfg(feature = "search")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(any(feature = "static_output", feature = "threads_output"))]
pub(crate) fn polling(
    evtx: &Sender<Event>,
    ps: &Arc<Mutex<PagerState>>,
    #[cfg(feature = "search")] input_thread_running: &Arc<AtomicBool>,
) -> Result<(), MinusError> {
    use crossterm::event;
    loop {
        #[cfg(feature = "search")]
        if !input_thread_running.load(Ordering::Relaxed) {
            continue;
        }
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| MinusError::HandleEvent(e.into()))?
        {
            let ev = event::read().map_err(|e| MinusError::HandleEvent(e.into()))?;
            let mut guard = ps.lock().unwrap();
            // Get the events
            let input = guard.input_classifier.classify_input(ev, &guard);
            if let Some(iev) = input {
                if let InputEvent::Number(n) = iev {
                    guard.prefix_num.push(n);
                    continue;
                }
                guard.prefix_num.clear();
                if let Err(TrySendError::Disconnected(_)) = evtx.try_send(Event::UserInput(iev)) {
                    break;
                }
            } else {
                guard.prefix_num.clear();
            }
        }
    }
    Result::<(), MinusError>::Ok(())
}

#[cfg(feature = "async_output")]
pub(crate) async fn streaming(
    evtx: Sender<Event>,
    ps: Arc<Mutex<PagerState>>,
    #[cfg(feature = "search")] input_thread_running: Arc<AtomicBool>,
) -> Result<(), MinusError> {
    use crossterm::event::EventStream;
    use futures_lite::stream::StreamExt;

    let mut stream = EventStream::new();
    loop {
        #[cfg(feature = "search")]
        if !input_thread_running.load(Ordering::Relaxed) {
            continue;
        }
        if let Some(ev) = stream.try_next().await? {
            let mut guard = ps.lock().unwrap();
            // Get the events
            let input = guard.input_classifier.classify_input(ev, &guard);
            if let Some(iev) = input {
                if let InputEvent::Number(n) = iev {
                    guard.prefix_num.push(n);
                    continue;
                }
                guard.prefix_num.clear();
                if let Err(TrySendError::Disconnected(_)) = evtx.try_send(Event::UserInput(iev)) {
                    break;
                }
            } else {
                guard.prefix_num.clear();
            }
        }
    }
    Result::<(), MinusError>::Ok(())
}
