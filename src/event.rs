use color_eyre::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum Event {
    Tick,
    Key(KeyEvent),
    #[allow(dead_code)]
    Mouse(MouseEvent),
    #[allow(dead_code)]
    Resize(u16, u16),
}

pub struct EventHandler {
    receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handler_sender = sender.clone();

        thread::spawn(move || {
            loop {
                if event::poll(tick_rate).unwrap_or(false) {
                    let res = match event::read() {
                        Ok(CrosstermEvent::Key(e)) => handler_sender.send(Event::Key(e)),
                        Ok(CrosstermEvent::Mouse(e)) => handler_sender.send(Event::Mouse(e)),
                        Ok(CrosstermEvent::Resize(w, h)) => {
                            handler_sender.send(Event::Resize(w, h))
                        }
                        _ => Ok(()),
                    };
                    if res.is_err() {
                        break;
                    }
                } else if handler_sender.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        Self { receiver }
    }

    pub fn next(&self) -> Result<Event> {
        Ok(self.receiver.recv()?)
    }
}
