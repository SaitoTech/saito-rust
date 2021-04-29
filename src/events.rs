use std::rc::Weak;

struct Event;

trait Observable {
    fn register(&mut self, observer: Weak<dyn Observer>);
}

trait Observer {
    fn notify(&self, event: &Event);
}