#![deny(warnings)]
#![feature(const_fn)]
#![feature(shared)]
#![feature(unsize)]
#![no_std]

extern crate untagged_option;

use core::marker::{PhantomData, Unsize};
use core::ptr::{self, Shared};

use untagged_option::UntaggedOption;

pub struct RingBuffer<T, A>
where
    A: Unsize<[T]>,
{
    head: usize,
    tail: usize,
    array: UntaggedOption<A>,
    _marker: PhantomData<[T]>,
}

impl<T, A> RingBuffer<T, A>
where
    A: Unsize<[T]>,
{
    pub const fn new() -> RingBuffer<T, A> {
        RingBuffer {
            array: UntaggedOption::none(),
            head: 0,
            tail: 0,
            _marker: PhantomData,
        }
    }

    pub fn spsc(&'static mut self) -> (Producer<T, A>, Consumer<T, A>) {
        let ptr = self as *mut _;
        unsafe {
            (
                Producer {
                    rb: Shared::new_unchecked(ptr),
                },
                Consumer {
                    rb: Shared::new_unchecked(ptr),
                },
            )
        }
    }
}

// Semantically, the `Consumer` owns the `tail` of the circular buffer
pub struct Consumer<T, A>
where
    A: Unsize<[T]>,
{
    rb: Shared<RingBuffer<T, A>>,
}

unsafe impl<T, A> Send for Consumer<T, A>
where
    A: Unsize<[T]>,
{
}

impl<T, A> Consumer<T, A>
where
    A: Unsize<[T]>,
{
    pub fn dequeue(&mut self) -> Option<T> {
        let rb = unsafe { self.rb.as_mut() };
        let array: &[T] = unsafe { &rb.array.some };
        let capacity = array.len();

        // NOTE(volatile) we don't own `head`; it can change at any time (e.g. in an interrupt).
        // volatile is used to rely this information to the compiler.
        if unsafe { ptr::read_volatile(&rb.head) } != rb.tail {
            // NOTE(ptr::read) don't "move out" the data. i.e. don't modify the drop flag associated
            // to the array element.
            let e = unsafe { ptr::read(array.get_unchecked(rb.tail)) };
            rb.tail = (rb.tail + 1) % capacity;
            Some(e)
        } else {
            None
        }
    }
}

// Semantically, the `Producer` "owns" the `head` of the circular buffer
pub struct Producer<T, A>
where
    A: Unsize<[T]>,
{
    rb: Shared<RingBuffer<T, A>>,
}

unsafe impl<T, A> Send for Producer<T, A>
where
    A: Unsize<[T]>,
{
}

impl<T, A> Producer<T, A>
where
    A: Unsize<[T]>,
{
    pub fn enqueue(&mut self, elem: T) -> Result<(), ()> {
        let rb = unsafe { self.rb.as_mut() };
        let array: &mut [T] = unsafe { &mut rb.array.some };
        let capacity = array.len();

        // NOTE(volatile) we don't own `tail`; it can change at any time (e.g. in an interrupt).
        // volatile is used to rely this information to the compiler.
        let next_head = (rb.head + 1) % capacity;
        if next_head != unsafe { ptr::read_volatile(&rb.tail) } {
            // NOTE(ptr::write) the slot we are about to write into is semantically empty so no need
            // to run a destructor on its current contents
            unsafe {
                ptr::write(array.get_unchecked_mut(rb.head), elem);
            }
            rb.head = next_head;
            Ok(())
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {

}
