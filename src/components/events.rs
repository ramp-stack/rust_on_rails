use super::{Context};
pub use crate::base::{MouseState, KeyboardState, NamedKey, Key, SmolStr};

use downcast_rs::{DowncastSync, impl_downcast};

use std::fmt::Debug;

pub type Events = std::collections::VecDeque<Box<dyn Event>>;

pub trait OnEvent: Debug {
    fn on_event(&mut self, _ctx: &mut Context, _event: &mut dyn Event) -> bool {true}
}

pub trait Event: Debug + DowncastSync {
    ///Function for event to decide on weather to pass the event to a child, Event can also be modified for the child
    fn pass(self: Box<Self>, _ctx: &mut Context, children: Vec<((f32, f32), (f32, f32))>) -> Vec<Option<Box<dyn Event>>>;
}
impl_downcast!(sync Event); 

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseEvent {
    pub position: Option<(f32, f32)>,
    pub state: MouseState,
}

impl Event for MouseEvent {
    fn pass(self: Box<Self>, _ctx: &mut Context, children: Vec<((f32, f32), (f32, f32))>) -> Vec<Option<Box<dyn Event>>> {
        let mut passed = false;
        children.into_iter().rev().map(|(offset, size)| {//Reverse to click on the top most element
            let position = self.position.and_then(|position| (!passed).then(|| (
                position.0 > offset.0 &&
                position.0 < offset.0+size.0 &&
                 position.1 > offset.1 &&
                position.1 < offset.1+size.1
                ).then(|| {
                    passed = true;
                    (position.0 - offset.0, position.1 - offset.1)
            })).flatten());
            Some(Box::new(MouseEvent{position, state: self.state}) as Box<dyn Event>)
        }).collect::<Vec<_>>().into_iter().rev().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardEvent {
    pub key: Key,
    pub state: KeyboardState,
}

impl Event for KeyboardEvent {
    fn pass(self: Box<Self>, _ctx: &mut Context, children: Vec<((f32, f32), (f32, f32))>) -> Vec<Option<Box<dyn Event>>> {
        children.into_iter().map(|_| Some(self.clone() as Box<dyn Event>)).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TickEvent;
impl Event for TickEvent {
    fn pass(self: Box<Self>, _ctx: &mut Context, children: Vec<((f32, f32), (f32, f32))>) -> Vec<Option<Box<dyn Event>>> {
        children.into_iter().map(|_| Some(Box::new(*self) as Box<dyn Event>)).collect()
    }
}
