//! implements flexcomm interface wrapper for easier usage across modules

use core::sync::atomic::{AtomicU8, Ordering};

use paste::paste;

use crate::clocks::{disable, enable_and_reset, SysconPeripheral};
use crate::peripherals::{
    FLEXCOMM0, FLEXCOMM1, FLEXCOMM14, FLEXCOMM15, FLEXCOMM2, FLEXCOMM3, FLEXCOMM4, FLEXCOMM5, FLEXCOMM6, FLEXCOMM7,
};
use crate::{pac, PeripheralType};

/// clock selection option
#[derive(Copy, Clone, Debug)]
pub enum Clock {
    /// SFRO
    Sfro,

    /// FFRO
    Ffro,

    /// `AUDIO_PLL`
    AudioPll,

    /// MASTER
    Master,

    /// FCn_FRG with Main clock source
    FcnFrgMain,

    /// FCn_FRG with Pll clock source
    FcnFrgPll,

    /// FCn_FRG with Sfro clock source
    FcnFrgSfro,

    /// FCn_FRG with Ffro clock source
    FcnFrgFfro,

    /// disabled
    None,
}

/// do not allow implementation of trait outside this mod
mod sealed {
    /// trait does not get re-exported outside flexcomm mod, allowing us to safely expose only desired APIs
    pub trait Sealed {}
}

struct State {
    refcount: AtomicU8,
}

impl State {
    const fn new() -> Self {
        Self {
            refcount: AtomicU8::new(0),
        }
    }
}

/// A generic reference to a usage of a Flexcomm peripheral.
///
/// Can be cloned to share the usage of the peripheral across multiple sites.
/// Dropping the last reference will disable the Flexcomm peripheral.
#[must_use]
pub(crate) struct FlexcommRef {
    disable_fn: fn(),
    state: &'static State,
}

impl FlexcommRef {
    fn new<T: FlexcommLowLevel>() -> Self {
        let state = T::state();
        assert_eq!(state.refcount.fetch_add(1, Ordering::AcqRel), 0);
        Self {
            disable_fn: T::disable,
            state,
        }
    }
}

impl Clone for FlexcommRef {
    fn clone(&self) -> Self {
        self.state.refcount.fetch_add(1, Ordering::AcqRel);
        Self {
            disable_fn: self.disable_fn,
            state: self.state,
        }
    }
}

impl Drop for FlexcommRef {
    fn drop(&mut self) {
        if self.state.refcount.fetch_sub(1, Ordering::AcqRel) == 1 {
            (self.disable_fn)();
        }
    }
}

/// primary low-level flexcomm interface
pub(crate) trait FlexcommLowLevel: sealed::Sealed + PeripheralType + SysconPeripheral + 'static + Send {
    // fetch the flexcomm register block for direct manipulation
    fn reg() -> &'static pac::flexcomm0::RegisterBlock;

    // set the clock select for this flexcomm instance and remove from reset
    fn enable(clk: Clock) -> FlexcommRef;

    // deconfigure the clock select
    fn disable();

    // a state associated with a flexcomm device, keeping count
    #[allow(private_interfaces)]
    fn state() -> &'static State;
}

macro_rules! impl_flexcomm {
    ($($idx:expr),*) => {
        $(
            paste!{
                impl sealed::Sealed for crate::peripherals::[<FLEXCOMM $idx>] {}

                impl FlexcommLowLevel for crate::peripherals::[<FLEXCOMM $idx>] {
                    fn reg() -> &'static crate::pac::flexcomm0::RegisterBlock {
                        // SAFETY: safe from single executor, enforce
                        // via peripheral reference lifetime tracking
                        unsafe {
                            &*crate::pac::[<Flexcomm $idx>]::ptr()
                        }
                    }

                    fn enable(clk: Clock) -> FlexcommRef {
                        // SAFETY: safe from single executor
                        let clkctl1 = unsafe { crate::pac::Clkctl1::steal() };

                        clkctl1.flexcomm($idx).fcfclksel().write(|w| match clk {
                            Clock::Sfro => w.sel().sfro_clk(),
                            Clock::Ffro => w.sel().ffro_clk(),
                            Clock::AudioPll => w.sel().audio_pll_clk(),
                            Clock::Master => w.sel().master_clk(),
                            Clock::FcnFrgMain => w.sel().fcn_frg_clk(),
                            Clock::FcnFrgPll => w.sel().fcn_frg_clk(),
                            Clock::FcnFrgSfro => w.sel().fcn_frg_clk(),
                            Clock::FcnFrgFfro => w.sel().fcn_frg_clk(),
                            Clock::None => w.sel().none(), // no clock? throw an error?
                        });

                        clkctl1.flexcomm($idx).frgclksel().write(|w| match clk {
                            Clock::FcnFrgMain => w.sel().main_clk(),
                            Clock::FcnFrgPll => w.sel().frg_pll_clk(),
                            Clock::FcnFrgSfro => w.sel().sfro_clk(),
                            Clock::FcnFrgFfro => w.sel().ffro_clk(),
                            _ => w.sel().none(),    // not using frg ...
                        });

                        // todo: add support for frg div/mult
                        clkctl1
                            .flexcomm($idx)
                            .frgctl()
                            .write(|w|
                            // SAFETY: unsafe only used for .bits() call
                            unsafe { w.mult().bits(0) });

                        enable_and_reset::<[<FLEXCOMM $idx>]>();

                        FlexcommRef::new::<Self>()
                    }

                    fn disable() {
                        // SAFETY: safe from single executor
                        let clkctl1 = unsafe { crate::pac::Clkctl1::steal() };
                        clkctl1.flexcomm($idx).fcfclksel().write(|w| w.sel().none());
                        clkctl1.flexcomm($idx).frgclksel().write(|w| w.sel().none());
                        disable::<[<FLEXCOMM $idx>]>();
                    }

                    #[allow(private_interfaces)]
                    fn state() -> &'static State {
                        static STATE: State = State::new();
                        &STATE
                    }
                }
            }
        )*
    }
}

impl_flexcomm!(0, 1, 2, 3, 4, 5, 6, 7);

// TODO: FLEXCOMM 14 is untested. Enable SPI support on FLEXCOMM14
// Add special case FLEXCOMM14
impl sealed::Sealed for crate::peripherals::FLEXCOMM14 {}

impl FlexcommLowLevel for crate::peripherals::FLEXCOMM14 {
    fn reg() -> &'static crate::pac::flexcomm0::RegisterBlock {
        // SAFETY: safe from single executor, enforce
        // via peripheral reference lifetime tracking
        unsafe { &*crate::pac::Flexcomm14::ptr() }
    }

    fn enable(clk: Clock) -> FlexcommRef {
        // SAFETY: safe from single executor
        let clkctl1 = unsafe { crate::pac::Clkctl1::steal() };

        clkctl1.fc14fclksel().write(|w| match clk {
            Clock::Sfro => w.sel().sfro_clk(),
            Clock::Ffro => w.sel().ffro_clk(),
            Clock::AudioPll => w.sel().audio_pll_clk(),
            Clock::Master => w.sel().master_clk(),
            Clock::FcnFrgMain => w.sel().fcn_frg_clk(),
            Clock::FcnFrgPll => w.sel().fcn_frg_clk(),
            Clock::FcnFrgSfro => w.sel().fcn_frg_clk(),
            Clock::FcnFrgFfro => w.sel().fcn_frg_clk(),
            Clock::None => w.sel().none(), // no clock? throw an error?
        });

        clkctl1.frg14clksel().write(|w| match clk {
            Clock::FcnFrgMain => w.sel().main_clk(),
            Clock::FcnFrgPll => w.sel().frg_pll_clk(),
            Clock::FcnFrgSfro => w.sel().sfro_clk(),
            Clock::FcnFrgFfro => w.sel().ffro_clk(),
            _ => w.sel().none(), // not using frg ...
        });

        // todo: add support for frg div/mult
        clkctl1.frg14ctl().write(|w|
                // SAFETY: unsafe only used for .bits() call
                unsafe { w.mult().bits(0) });

        enable_and_reset::<FLEXCOMM14>();

        FlexcommRef::new::<Self>()
    }

    fn disable() {
        // SAFETY: safe from single executor
        let clkctl1 = unsafe { crate::pac::Clkctl1::steal() };
        clkctl1.fc14fclksel().write(|w| w.sel().none());
        clkctl1.frg14clksel().write(|w| w.sel().none());
        disable::<FLEXCOMM14>();
    }

    #[allow(private_interfaces)]
    fn state() -> &'static State {
        static STATE: State = State::new();
        &STATE
    }
}

// Add special case FLEXCOMM15
impl sealed::Sealed for crate::peripherals::FLEXCOMM15 {}

impl FlexcommLowLevel for crate::peripherals::FLEXCOMM15 {
    fn reg() -> &'static crate::pac::flexcomm0::RegisterBlock {
        // SAFETY: safe from single executor, enforce
        // via peripheral reference lifetime tracking
        unsafe { &*crate::pac::Flexcomm15::ptr() }
    }

    fn enable(clk: Clock) -> FlexcommRef {
        // SAFETY: safe from single executor
        let clkctl1 = unsafe { crate::pac::Clkctl1::steal() };

        clkctl1.fc15fclksel().write(|w| match clk {
            Clock::Sfro => w.sel().sfro_clk(),
            Clock::Ffro => w.sel().ffro_clk(),
            Clock::AudioPll => w.sel().audio_pll_clk(),
            Clock::Master => w.sel().master_clk(),
            Clock::FcnFrgMain => w.sel().fcn_frg_clk(),
            Clock::FcnFrgPll => w.sel().fcn_frg_clk(),
            Clock::FcnFrgSfro => w.sel().fcn_frg_clk(),
            Clock::FcnFrgFfro => w.sel().fcn_frg_clk(),
            Clock::None => w.sel().none(), // no clock? throw an error?
        });
        clkctl1.frg15clksel().write(|w| match clk {
            Clock::FcnFrgMain => w.sel().main_clk(),
            Clock::FcnFrgPll => w.sel().frg_pll_clk(),
            Clock::FcnFrgSfro => w.sel().sfro_clk(),
            Clock::FcnFrgFfro => w.sel().ffro_clk(),
            _ => w.sel().none(), // not using frg ...
        });
        // todo: add support for frg div/mult
        clkctl1.frg15ctl().write(|w|
                // SAFETY: unsafe only used for .bits() call
                unsafe { w.mult().bits(0) });

        enable_and_reset::<FLEXCOMM15>();

        FlexcommRef::new::<Self>()
    }

    fn disable() {
        // SAFETY: safe from single executor
        let clkctl1 = unsafe { crate::pac::Clkctl1::steal() };
        clkctl1.fc15fclksel().write(|w| w.sel().none());
        clkctl1.frg15clksel().write(|w| w.sel().none());
        disable::<FLEXCOMM15>();
    }

    #[allow(private_interfaces)]
    fn state() -> &'static State {
        static STATE: State = State::new();
        &STATE
    }
}

macro_rules! into_mode {
    ($mode:ident, $($fc:ident),*) => {
        paste! {
            /// Sealed Mode trait
            trait [<SealedInto $mode:camel>]: FlexcommLowLevel {}

            /// Select mode of operation
            #[allow(private_bounds)]
            pub trait [<Into $mode:camel>]: [<SealedInto $mode:camel>] {
                /// Set mode of operation
                fn [<into_ $mode>]() {
                    Self::reg().pselid().write(|w| w.persel().[<$mode>]());
                }
            }
        }

        $(
            paste!{
                impl [<SealedInto $mode:camel>] for crate::peripherals::$fc {}
                impl [<Into $mode:camel>] for crate::peripherals::$fc {}
            }
        )*
    }
}

into_mode!(usart, FLEXCOMM0, FLEXCOMM1, FLEXCOMM2, FLEXCOMM3, FLEXCOMM4, FLEXCOMM5, FLEXCOMM6, FLEXCOMM7);

into_mode!(spi, FLEXCOMM0, FLEXCOMM1, FLEXCOMM2, FLEXCOMM3, FLEXCOMM4, FLEXCOMM5, FLEXCOMM6, FLEXCOMM7, FLEXCOMM14);

into_mode!(i2c, FLEXCOMM0, FLEXCOMM1, FLEXCOMM2, FLEXCOMM3, FLEXCOMM4, FLEXCOMM5, FLEXCOMM6, FLEXCOMM7, FLEXCOMM15);

into_mode!(
    i2s_transmit,
    FLEXCOMM0,
    FLEXCOMM1,
    FLEXCOMM2,
    FLEXCOMM3,
    FLEXCOMM4,
    FLEXCOMM5,
    FLEXCOMM6,
    FLEXCOMM7
);

into_mode!(
    i2s_receive,
    FLEXCOMM0,
    FLEXCOMM1,
    FLEXCOMM2,
    FLEXCOMM3,
    FLEXCOMM4,
    FLEXCOMM5,
    FLEXCOMM6,
    FLEXCOMM7
);
