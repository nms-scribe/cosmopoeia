use std::time::Duration;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;

pub trait ProgressObserver {

    // the parameters are passed as callbacks in case the progress implementation doesn't care (such as if its Option<ProgressObserver>::None)
    fn start<Message: AsRef<str>, Callback: Fn() -> (Message,Option<usize>)>(&mut self, callback: Callback);

    fn update<Callback: Fn() -> usize>(&self, callback: Callback);

    fn message<Message: AsRef<str>, Callback: Fn() -> Message>(&self, callback: Callback);

    fn finish<Message: AsRef<str>, Callback: Fn() -> Message>(&mut self, callback: Callback);

}

impl<OtherProgressObserver: ProgressObserver> ProgressObserver for Option<&mut OtherProgressObserver> {

    fn start<Message: AsRef<str>, Callback: Fn() -> (Message,Option<usize>)>(&mut self, callback: Callback) {
        if let Some(me) = self {
            me.start(callback)
        }
    }

    fn update<Callback: Fn() -> usize>(&self, callback: Callback) {
        if let Some(me) = self {
            me.update(callback)
        }
    }

    fn message<Message: AsRef<str>, Callback: Fn() -> Message>(&self, callback: Callback) {
        if let Some(me) = self {
            me.message(callback)
        }
    }

    fn finish<Message: AsRef<str>, Callback: Fn() -> Message>(&mut self, callback: Callback) {
        if let Some(me) = self {
            me.finish(callback)
        }
    }
}

pub struct ConsoleProgressBar {

    bar: Option<ProgressBar>

}

impl ConsoleProgressBar {

    pub fn new() -> Self {
        Self {
            bar: None
        }
    }

}

impl ProgressObserver for ConsoleProgressBar {

    fn start<Message: AsRef<str>, Callback: Fn() -> (Message,Option<usize>)>(&mut self, callback: Callback) {
        let (message,step_count) = callback();
        if let Some(bar) = &self.bar {
            bar.reset();
            if let Some(step_count) = step_count {
                bar.set_length(step_count as u64);
                bar.disable_steady_tick();
            } else {
                bar.enable_steady_tick(Duration::new(1,0));
            }
            bar.set_message(message.as_ref().to_owned());
            bar.set_position(0);
        } else {
            let bar = if let Some(step_count) = step_count {
                ProgressBar::new(step_count as u64)
            } else {
                let bar = ProgressBar::new_spinner();
                bar.enable_steady_tick(Duration::new(1,0)); // allows the spinner to update even though progress isn't happening.
                bar
            };
            // FUTURE: I should make this look different when it's a spinner...
            bar.set_style(ProgressStyle::with_template("({elapsed_precise}) [{bar:40}] [ETA: {eta_precise}] {msg} {spinner}")
                .unwrap()
                .progress_chars("=>-"));
            bar.set_message(message.as_ref().to_owned());
            self.bar = Some(bar);
        }

    }

    fn update<Callback: Fn() -> usize>(&self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.set_position(callback() as u64);
        }
    }

    fn message<Message: AsRef<str>, Callback: Fn() -> Message>(&self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.set_message(callback().as_ref().to_owned())
        }
    }

    fn finish<Message: AsRef<str>, Callback: Fn() -> Message>(&mut self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.finish_with_message(callback().as_ref().to_owned());
            self.bar = None;
        }
    }
}


