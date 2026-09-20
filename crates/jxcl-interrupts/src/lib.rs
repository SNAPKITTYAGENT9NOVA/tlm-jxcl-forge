//! A deterministic external-interrupt injection mechanism (priority queue + mask flag)
//! for embedding jxcl in a simulator or RPC host.
//!
//! Owns: InterruptController: queue/mask/deliver. Provides a priority-based interrupt
//! queue that can be checked at each instruction boundary to determine if any
//! interrupts should be delivered to the machine.
#![forbid(unsafe_code)]

use std::collections::VecDeque;

/// An interrupt with a priority level and identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Interrupt {
    pub priority: u8,
    pub id: u16,
}

impl Interrupt {
    /// Create a new interrupt with the given priority and id.
    pub const fn new(priority: u8, id: u16) -> Self {
        Interrupt { priority, id }
    }
}

/// Manages external interrupts for the machine: queuing, masking, and delivery.
/// Interrupts are processed in priority order (higher priority first).
#[derive(Debug, Clone)]
pub struct InterruptController {
    queue: VecDeque<Interrupt>,
    mask: bool,
}

impl InterruptController {
    /// Create a new interrupt controller with the queue enabled (mask = false).
    pub fn new() -> Self {
        InterruptController {
            queue: VecDeque::new(),
            mask: false,
        }
    }

    /// Inject an interrupt into the queue.
    pub fn inject(&mut self, interrupt: Interrupt) {
        self.queue.push_back(interrupt);
        // Keep the queue sorted by priority (higher priority first)
        self.sort_queue();
    }

    /// Check if the interrupt queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get the number of pending interrupts.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Deliver the next interrupt if one is available and interrupts are enabled.
    /// Returns Some(interrupt) if delivery occurred, None otherwise.
    pub fn deliver(&mut self) -> Option<Interrupt> {
        if self.mask || self.queue.is_empty() {
            return None;
        }
        self.queue.pop_front()
    }

    /// Enable the interrupt mask (disable interrupt delivery).
    pub fn set_mask(&mut self, masked: bool) {
        self.mask = masked;
    }

    /// Check if interrupts are currently masked.
    pub fn is_masked(&self) -> bool {
        self.mask
    }

    /// Clear all pending interrupts.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Sort the queue by priority (higher priority first).
    fn sort_queue(&mut self) {
        // For a simple VecDeque, we'll use a stable sort approach
        // Convert to a vector, sort, and rebuild
        let mut vec: Vec<_> = self.queue.iter().copied().collect();
        vec.sort_by(|a, b| b.priority.cmp(&a.priority)); // Higher priority first
        self.queue = vec.into_iter().collect();
    }
}

impl Default for InterruptController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_creation() {
        let intr = Interrupt::new(5, 10);
        assert_eq!(intr.priority, 5);
        assert_eq!(intr.id, 10);
    }

    #[test]
    fn interrupt_ordering() {
        let intr1 = Interrupt::new(1, 100);
        let intr2 = Interrupt::new(5, 200);
        assert!(intr2 > intr1);
    }

    #[test]
    fn interrupt_controller_empty() {
        let controller = InterruptController::new();
        assert!(controller.is_empty());
        assert_eq!(controller.pending_count(), 0);
    }

    #[test]
    fn interrupt_injection() {
        let mut controller = InterruptController::new();
        assert!(controller.is_empty());

        controller.inject(Interrupt::new(5, 100));
        assert!(!controller.is_empty());
        assert_eq!(controller.pending_count(), 1);
    }

    #[test]
    fn interrupt_delivery() {
        let mut controller = InterruptController::new();
        controller.inject(Interrupt::new(5, 100));

        let intr = controller.deliver();
        assert_eq!(intr, Some(Interrupt::new(5, 100)));
        assert!(controller.is_empty());
    }

    #[test]
    fn interrupt_masking() {
        let mut controller = InterruptController::new();
        assert!(!controller.is_masked());

        controller.set_mask(true);
        assert!(controller.is_masked());

        controller.inject(Interrupt::new(5, 100));
        // Should not deliver while masked
        assert!(controller.deliver().is_none());

        controller.set_mask(false);
        // Now should deliver
        assert_eq!(controller.deliver(), Some(Interrupt::new(5, 100)));
    }

    #[test]
    fn interrupt_priority_ordering() {
        let mut controller = InterruptController::new();
        controller.inject(Interrupt::new(1, 100));
        controller.inject(Interrupt::new(10, 200));
        controller.inject(Interrupt::new(5, 300));

        // Should deliver in priority order (highest first)
        assert_eq!(controller.deliver().map(|i| i.priority), Some(10));
        assert_eq!(controller.deliver().map(|i| i.priority), Some(5));
        assert_eq!(controller.deliver().map(|i| i.priority), Some(1));
    }

    #[test]
    fn interrupt_controller_clear() {
        let mut controller = InterruptController::new();
        controller.inject(Interrupt::new(5, 100));
        controller.inject(Interrupt::new(3, 200));
        assert_eq!(controller.pending_count(), 2);

        controller.clear();
        assert_eq!(controller.pending_count(), 0);
        assert!(controller.is_empty());
    }

    #[test]
    fn interrupt_controller_determinism() {
        let mut controller1 = InterruptController::new();
        let mut controller2 = InterruptController::new();

        for i in 0..10 {
            controller1.inject(Interrupt::new(i, i as u16));
            controller2.inject(Interrupt::new(i, i as u16));
        }

        while let (Some(i1), Some(i2)) = (controller1.deliver(), controller2.deliver()) {
            assert_eq!(i1, i2);
        }
    }
}
