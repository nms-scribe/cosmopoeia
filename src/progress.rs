use std::time::Duration;
use std::iter::Enumerate;

use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use priority_queue::PriorityQueue;


pub(crate) trait ProgressObserver {

    // TODO: What if the 'start' methods returned some sort of object that would handle the rest of the information?
    // - the object would guarantee that the progress observer was already borrowed for mutability, preventing
    //   updates from multiple items.
    // - if the start also included the 'finish' message, the object could be passed to the to_vec methods instead
    //   without requiring the 'Feature::LAYER_NAME' requirement, allowing me to implement the progress watching
    //   for any iterator.
    // - in fact, such a progress thingie might have a method which quickly wraps the iterator, calling the
    //   enumerator function, so it doesn't have to be called automatically in the code.

    // the parameters are passed as callbacks in case the progress implementation doesn't care (such as if its Option<ProgressObserver>::None)
    fn start_known_endpoint<Message: AsRef<str>, Callback: FnOnce() -> (Message,usize)>(&mut self, callback: Callback);

    fn start_unknown_endpoint<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback);

    fn start<Message: AsRef<str>, Callback: FnOnce() -> (Message,Option<usize>)>(&mut self, callback: Callback);

    fn update<Callback: FnOnce() -> usize>(&self, callback: Callback);

    fn update_step_length<Callback: FnOnce() -> usize>(&self, callback: Callback);

    fn message<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback);

    fn warning<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback);

    fn finish<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, callback: Callback);

}


// This one allows for not observing when you don't need it.
impl ProgressObserver for () {

    fn start_known_endpoint<Message: AsRef<str>, Callback: FnOnce() -> (Message,usize)>(&mut self, _: Callback) {
    }

    fn start_unknown_endpoint<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, _: Callback) {
    }

    fn start<Message: AsRef<str>, Callback: FnOnce() -> (Message,Option<usize>)>(&mut self, _: Callback) {
    }

    fn update<Callback: FnOnce() -> usize>(&self, _: Callback) {
    }

    fn update_step_length<Callback: FnOnce() -> usize>(&self, _: Callback) {
    }

    fn message<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, _: Callback) {
    }

    fn warning<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, _: Callback){

    }

    fn finish<Message: AsRef<str>, Callback: FnOnce() -> Message>(&mut self, _: Callback) {
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

    pub(crate) fn announce(&self, message: &str) {
        let message = format!("== {} ==",message);
        if let Some(bar) = &self.bar {
            bar.println(message)
        } else {
            println!("{}",message)
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

    fn update_step_length<Callback: FnOnce() -> usize>(&self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.set_length(callback() as u64);
        }
    }



    fn message<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback) {
        if let Some(bar) = &self.bar {
            bar.set_message(callback().as_ref().to_owned())
        }
    }

    fn warning<Message: AsRef<str>, Callback: FnOnce() -> Message>(&self, callback: Callback){
        // FUTURE: Make this in another color?
        // TODO: Test this to make sure it's working...
        if let Some(bar) = &self.bar {
            bar.println(callback())
        } else {
            eprintln!("{}",callback().as_ref())
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

pub(crate) struct IteratorWatcher<'progress,Message: AsRef<str>, Progress: ProgressObserver, IteratorType> {
    finish: Message,
    progress: &'progress mut Progress,
    inner: Enumerate<IteratorType>
}

impl<'watcher,Message: AsRef<str>, Progress: ProgressObserver, ItemType, IteratorType: Iterator<Item=ItemType>> Iterator for IteratorWatcher<'watcher,Message,Progress,IteratorType> {

    type Item = ItemType;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i,next)) = self.inner.next() {
            self.progress.update(|| i);
            Some(next)
        } else {
            self.progress.finish(|| &self.finish);
            None
        }
        
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()        
    }
    

}

pub(crate) trait WatchableIterator: Iterator + Sized {

    fn watch<'progress, StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &'progress mut Progress, start: StartMessage, finish: FinishMessage) -> IteratorWatcher<FinishMessage, Progress, Self>;
}

impl<IteratorType: Iterator> WatchableIterator for IteratorType {

    // TODO: This takes care of a large number of patterns. The ones it doesn't handle are:
    // - patterns which deal with popping items off a queue -- see below
    // - patterns where we don't know an endpoint -- might be handled with a macro_rule wrapping the section.
    // As far as the queues go, except for the part where we have multiple queues (calculating shore distance),
    // I could have something that wraps a queue, or vec, and watches as things are removed and added, changing the step_length of the progress
    // bar as well as updating the current step.

    fn watch<'progress, StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &'progress mut Progress, start: StartMessage, finish: FinishMessage) -> IteratorWatcher<FinishMessage, Progress, Self> {
        progress.start(|| (start,self.size_hint().1));
        IteratorWatcher { 
            finish: finish, 
            progress: progress, 
            inner: self.enumerate()
        }

    }


}

pub(crate) struct QueueWatcher<'progress,Message: AsRef<str>, Progress: ProgressObserver, ItemType> {
    finish: Message,
    progress: &'progress mut Progress,
    inner: Vec<ItemType>,
    popped: usize,
    pushed: usize,
}

impl<'progress,Message: AsRef<str>, Progress: ProgressObserver, ItemType> QueueWatcher<'progress,Message,Progress,ItemType> {

    pub(crate) fn pop(&mut self) -> Option<ItemType> {
        let result = self.inner.pop();
        self.popped += 1;
        let len = self.inner.len();
        if len == 0 {
            self.progress.finish(|| &self.finish)
        } else {
            self.progress.update(|| self.popped);
        }
        result
    }

    pub(crate) fn push(&mut self, value: ItemType) {
        self.inner.push(value);
        self.pushed += 1;
        self.progress.update_step_length(|| self.pushed);
    }

    pub(crate) fn last(&self) -> Option<&ItemType> {
        self.inner.last()
    }
} 

pub(crate) trait WatchableQueue<ItemType: Sized> {

    fn watch_queue<'progress, StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &'progress mut Progress, start: StartMessage, finish: FinishMessage) -> QueueWatcher<FinishMessage, Progress, ItemType>;
}

impl<ItemType> WatchableQueue<ItemType> for Vec<ItemType> {

    // TODO: This takes care of a large number of patterns. The ones it doesn't handle are:
    // - patterns which deal with popping items off a queue -- see below
    // - patterns where we don't know an endpoint -- might be handled with a macro_rule wrapping the section.
    // As far as the queues go, except for the part where we have multiple queues (calculating shore distance),
    // I could have something that wraps a queue, or vec, and watches as things are removed and added, changing the step_length of the progress
    // bar as well as updating the current step.

    fn watch_queue<'progress, StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &'progress mut Progress, start: StartMessage, finish: FinishMessage) -> QueueWatcher<FinishMessage, Progress, ItemType> {
        progress.start(|| (start,Some(self.len())));
        QueueWatcher { 
            finish: finish, 
            progress: progress, 
            inner: self,
            pushed: 0,
            popped: 0
        }

    }


}

pub(crate) struct PriorityQueueWatcher<'progress,Message: AsRef<str>, Progress: ProgressObserver, ItemType: std::hash::Hash + Eq, PriorityType: Ord> {
    finish: Message,
    progress: &'progress mut Progress,
    inner: PriorityQueue<ItemType,PriorityType>,
    popped: usize,
    pushed: usize,
}

impl<'progress,Message: AsRef<str>, Progress: ProgressObserver, ItemType: std::hash::Hash + Eq, PriorityType: Ord> PriorityQueueWatcher<'progress,Message,Progress,ItemType,PriorityType> {

    pub(crate) fn pop(&mut self) -> Option<(ItemType,PriorityType)> {
        let result = self.inner.pop();
        self.popped += 1;
        let len = self.inner.len();
        if len == 0 {
            self.progress.finish(|| &self.finish)
        } else {
            self.progress.update(|| self.popped);
        }
        result
    }

    pub(crate) fn push(&mut self, value: ItemType, priority: PriorityType) {
        self.inner.push(value,priority);
        self.pushed += 1;
        self.progress.update_step_length(|| self.pushed);
    }

} 

pub(crate) trait WatchablePriorityQueue<ItemType: std::hash::Hash + Eq, PriorityType: Ord> {

    fn watch_queue<'progress, StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &'progress mut Progress, start: StartMessage, finish: FinishMessage) -> PriorityQueueWatcher<FinishMessage, Progress, ItemType,PriorityType>;
}

impl<ItemType: std::hash::Hash + Eq, PriorityType: Ord> WatchablePriorityQueue<ItemType,PriorityType> for PriorityQueue<ItemType,PriorityType> {

    // TODO: This takes care of a large number of patterns. The ones it doesn't handle are:
    // - patterns which deal with popping items off a queue -- see below
    // - patterns where we don't know an endpoint -- might be handled with a macro_rule wrapping the section.
    // As far as the queues go, except for the part where we have multiple queues (calculating shore distance),
    // I could have something that wraps a queue, or vec, and watches as things are removed and added, changing the step_length of the progress
    // bar as well as updating the current step.

    fn watch_queue<'progress, StartMessage: AsRef<str>, FinishMessage: AsRef<str>, Progress: ProgressObserver>(self, progress: &'progress mut Progress, start: StartMessage, finish: FinishMessage) -> PriorityQueueWatcher<FinishMessage, Progress, ItemType,PriorityType> {
        progress.start(|| (start,Some(self.len())));
        PriorityQueueWatcher { 
            finish: finish, 
            progress: progress, 
            inner: self,
            pushed: 0,
            popped: 0
        }

    }


}


