#![doc(html_root_url = "https://docs.rs/rotary-encoder-hal/0.5.0")]
//! # rotary-encoder-hal
//!
//! A platform agnostic rotary encoder library
//!
//! Built using [`embedded-hal`] traits
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal/0.2

#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]

use either::Either;

#[cfg(not(feature = "embedded-hal-alpha"))]
use embedded_hal::digital::v2::InputPin;

#[cfg(feature = "embedded-hal-alpha")]
use embedded_hal_alpha::digital::blocking::InputPin;

/// Holds current/old state and both [`InputPin`](https://docs.rs/embedded-hal/0.2.3/embedded_hal/digital/v2/trait.InputPin.html)
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rotary<A, B> {
    pin_a: A,
    pin_b: B,
    state: u8,

    #[cfg(feature = "table-decoder")]
    prev_next: u8,
    #[cfg(feature = "table-decoder")]
    store: u16,
}

/// The encoder direction is either `Clockwise`, `CounterClockwise`, or `None`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Direction {
    /// A clockwise turn
    Clockwise,
    /// A counterclockwise turn
    CounterClockwise,
    /// No change
    None,
}

#[cfg(not(feature = "table-decoder"))]
impl From<u8> for Direction {
    fn from(s: u8) -> Self {
        match s {
            0b0001 | 0b0111 | 0b1000 | 0b1110 => Direction::Clockwise,
            0b0010 | 0b0100 | 0b1011 | 0b1101 => Direction::CounterClockwise,
            _ => Direction::None,
        }
    }
}

#[cfg(feature = "table-decoder")]
impl From<u16> for Direction {
    fn from(s: u16) -> Self {
        match s & 0x00ff {
            0x17 => Direction::Clockwise,
            0x2b => Direction::CounterClockwise,
            _ => Direction::None,
        }
    }
}

impl<A, B> Rotary<A, B>
where
    A: InputPin,
    B: InputPin,
{
    /// Accepts two [`InputPin`](https://docs.rs/embedded-hal/0.2.3/embedded_hal/digital/v2/trait.InputPin.html)s, these will be read on every `update()`
    pub fn new(pin_a: A, pin_b: B) -> Self {
        Self {
            pin_a,
            pin_b,
            state: 0u8,

            #[cfg(feature = "table-decoder")]
            prev_next: 0u8,
            #[cfg(feature = "table-decoder")]
            store: 0u16,
        }
    }

    #[cfg(not(feature = "table-decoder"))]
    /// Call `update` to evaluate the next state of the encoder, propagates errors from `InputPin` read
    pub fn update(&mut self) -> Result<Direction, Either<A::Error, B::Error>> {
        // use mask to get previous state value
        let mut s = self.state & 0b11;

        #[cfg(not(feature = "embedded-hal-alpha"))]
        let (a_is_low, b_is_low) = (self.pin_a.is_low(), self.pin_b.is_low());
        #[cfg(feature = "embedded-hal-alpha")]
        let (a_is_low, b_is_low) = (self.pin_a.is_low(), self.pin_b.is_low());

        // move in the new state
        if a_is_low.map_err(Either::Left)? {
            s |= 0b100;
        }
        if b_is_low.map_err(Either::Right)? {
            s |= 0b1000;
        }
        // move new state in
        self.state = s >> 2;
        Ok(s.into())
    }

    /// Returns a reference to the first pin. Can be used to clear interrupt.
    pub fn pin_a(&mut self) -> &mut A {
        &mut self.pin_a
    }

    /// Returns a reference to the second pin. Can be used to clear interrupt.
    pub fn pin_b(&mut self) -> &mut B {
        &mut self.pin_b
    }

    /// Consumes this `Rotary`, returning the underlying pins `A` and `B`.
    pub fn into_inner(self) -> (A, B) {
        (self.pin_a, self.pin_b)
    }

    #[cfg(feature = "table-decoder")]
    /// Call `update` to evaluate the next state of the encoder, propagates errors from `InputPin` read
    pub fn update(&mut self) -> Result<Direction, Either<A::Error, B::Error>> {
        #[cfg(not(feature = "embedded-hal-alpha"))]
        let (a_is_high, b_is_high) = (self.pin_a.is_high(), self.pin_b.is_low());
        #[cfg(feature = "embedded-hal-alpha")]
        let (a_is_high, b_is_high) = (self.pin_a.is_high(), self.pin_b.is_low());

        // Implemented after https://www.best-microcontroller-projects.com/rotary-encoder.html
        self.prev_next <<= 2;
        if a_is_high.map_err(Either::Left)? {
            self.prev_next |= 0x02;
        }
        if b_is_high.map_err(Either::Right)? {
            self.prev_next |= 0x01;
        }
        self.prev_next &= 0x0f;

        match self.prev_next {
            /*Invalid cases 0 | 3 | 5 | 6 | 9 | 10 | 12 | 15=>, */
            /*valid cases*/
            1 | 2 | 4 | 7 | 8 | 11 | 13 | 14 => {
                self.store <<= 4;
                self.store |= self.prev_next as u16;

                Ok(self.store.into())
            }
            _ => Ok(Direction::None),
        }
    }
}
