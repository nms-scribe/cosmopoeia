use std::time::Duration;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;

pub(crate) trait ProgressObserver {

    // the parameters are passed as callbacks in case the progress implementation doesn't care (such as if its Option<ProgressObserver>::None)
    fn start_known_endpoint<Message: AsRef<str>, Callback: FnOnce() -> (Message,usize)>(&mut self, callback: Callback);

    fn start_unknown_endpoint<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback);

    fn start<Message: AsRef<str>, Callback: FnOnce() -> (Message,Option<usize>)>(&mut self, callback: Callback);

    fn update<Callback: FnOnce() -> usize>(&self, callback: Callback);

    fn message<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback);

    fn finish<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback);

}

impl<OtherProgressObserver: ProgressObserver> ProgressObserver for Option<&mut OtherProgressObserver> {

    fn start_known_endpoint<Message: AsRef<str>, Callback: FnOnce() -> (Message,usize)>(&mut self, callback: Callback) {
        if let Some(me) = self {
            me.start_known_endpoint(callback)
        }
    }

    fn start_unknown_endpoint<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback) {
        if let Some(me) = self {
            me.start_unknown_endpoint(callback)
        }
    }

    fn start<Message: AsRef<str>, Callback: FnOnce() -> (Message,Option<usize>)>(&mut self, callback: Callback) {
        if let Some(me) = self {
            me.start(callback)
        }
    }



    fn update<Callback: FnOnce() -> usize>(&self, callback: Callback) {
        if let Some(me) = self {
            me.update(callback)
        }
    }

    fn message<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback) {
        if let Some(me) = self {
            me.message(callback)
        }
    }

    fn finish<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback) {
        if let Some(me) = self {
            me.finish(callback)
        }
    }
}

pub(crate) struct ConsoleProgressBar {

    bar: Option<ProgressBar>

}

impl ConsoleProgressBar {

    pub(crate) fn new() -> Self {
        Self {
            bar: None
        }
    }

    fn style_as_spinner(bar: &mut ProgressBar) {
        bar.enable_steady_tick(Duration::new(0,500));
        bar.set_style(ProgressStyle::with_template("({elapsed_precise}) {msg} {spinner}")
            .unwrap()
            //.tick_strings(SPINNER_STRINGS)
            //.tick_chars(SPINNER_CHARS)
        );

    }

    fn style_as_progress(bar: &mut ProgressBar) {
        bar.disable_steady_tick();
        bar.set_style(ProgressStyle::with_template("({elapsed_precise}) [{bar:40}] [ETA: {eta_precise}] {msg} {spinner}")
            .unwrap()
            //.tick_strings(SPINNER_STRINGS)
            //.tick_chars(SPINNER_CHARS)
            .progress_chars("=> ")
        );

    }

    fn style_as_finished(bar: &mut ProgressBar) {
        bar.set_style(ProgressStyle::with_template("({elapsed_precise}) {msg}")
            .unwrap());

    }

    fn start<Message: AsRef<str>>(&mut self, message: Message, step_count: Option<usize>) {
        if let Some(bar) = &mut self.bar {
            bar.reset();
            if let Some(step_count) = step_count {
                bar.set_length(step_count as u64);
                Self::style_as_progress(bar)
            } else {
                Self::style_as_spinner(bar);
            }
            bar.set_message(message.as_ref().to_owned());
        } else {
            let bar = if let Some(step_count) = step_count {
                let mut bar = ProgressBar::new(step_count as u64);
                Self::style_as_progress(&mut bar);
                bar
            } else {
                let mut bar = ProgressBar::new_spinner();
                Self::style_as_spinner(&mut bar);
                bar
            };
            // FUTURE: I should make this look different when it's a spinner...
            bar.set_message(message.as_ref().to_owned());
            self.bar = Some(bar);
        }

    }



}

impl ProgressObserver for ConsoleProgressBar {

    fn start_known_endpoint<Message: AsRef<str>, Callback: FnOnce() -> (Message,usize)>(&mut self, callback: Callback) {
        let (message,step_count) = callback();
        self.start(message, Some(step_count))
    }

    fn start_unknown_endpoint<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback) {
        self.start(callback(), None)
    }

    fn start<Message: AsRef<str>, Callback: FnOnce() -> (Message,Option<usize>)>(&mut self, callback: Callback) {
        let (message,step_count) = callback();
        self.start(message, step_count)
    }


    fn update<Callback: FnOnce() -> usize>(&self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.set_position(callback() as u64);
        }
    }

    fn message<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.set_message(callback().as_ref().to_owned())
        }
    }

    fn finish<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback) {
        if let Some(bar) = &mut self.bar {
            Self::style_as_finished(bar);
            bar.finish_with_message(callback().as_ref().to_owned());
            self.bar = None;
        }
    }

}


