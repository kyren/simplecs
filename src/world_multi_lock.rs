use std::any::TypeId;
use std::cell::RefCell;
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use failure::Error;

use component::Component;
use ecs::{ComponentReadHandle, ComponentWriteHandle};
use world::World;

/// Locks must be acquired in this order, resources before components, and in TypeId order.
#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub enum LockId {
    Resource(TypeId),
    Component(TypeId),
}

pub trait WorldLocker<'a> {
    type Handle;

    fn id(&self) -> LockId;
    fn lock(&self, world: &'a World) -> Result<(), Error>;
    // Will panic unless 'lock' has been called
    fn handle(self) -> Self::Handle;
}

pub trait WorldMultiLocker<'a> {
    type Handles;

    fn lockers<'b>(&'b self) -> Vec<(LockId, Box<FnMut(&'a World) -> Result<(), Error> + 'b>)>;
    // Will panic unless all locker methods have been called
    fn handles(self) -> Self::Handles;

    fn lock(self, world: &'a World) -> Result<Self::Handles, Error>
    where
        Self: Sized,
    {
        {
            let mut lockers = self.lockers();
            lockers.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, mut locker) in lockers {
                locker(world)?;
            }
        }

        Ok(self.handles())
    }
}

macro_rules! impl_tuple {
    ($($locker:ident)*) => (
        impl<'a, $($locker,)*> WorldMultiLocker<'a> for ($($locker,)*)
            where $($locker: WorldLocker<'a>,)*
        {
            type Handles = ($($locker::Handle,)*);

            #[allow(non_snake_case)]
            fn lockers<'b>(&'b self) -> Vec<(LockId, Box<FnMut(&'a World) -> Result<(), Error> + 'b>)> {
                let mut lockers = Vec::<(LockId, Box<FnMut(&'a World) -> Result<(), Error> + 'b>)>::new();
                let ($(ref $locker,)*) = *self;
                $(lockers.push(($locker.id(), Box::new(move |world| $locker.lock(world))));)*
                lockers
            }

            #[allow(non_snake_case)]
            fn handles(self) -> Self::Handles {
                let ($($locker,)*) = self;
                ($($locker.handle(),)*)
            }
        }
    );
}

impl_tuple!{A}
impl_tuple!{A B}
impl_tuple!{A B C}
impl_tuple!{A B C D}
impl_tuple!{A B C D E}
impl_tuple!{A B C D E F}
impl_tuple!{A B C D E F G}
impl_tuple!{A B C D E F G H}
impl_tuple!{A B C D E F G H I}
impl_tuple!{A B C D E F G H I J}
impl_tuple!{A B C D E F G H I J K}
impl_tuple!{A B C D E F G H I J K L}
impl_tuple!{A B C D E F G H I J K L M}
impl_tuple!{A B C D E F G H I J K L M N}
impl_tuple!{A B C D E F G H I J K L M N O}
impl_tuple!{A B C D E F G H I J K L M N O P}

impl World {
    /// Lock multiple resources and components from a World, but always lock them in the correct
    /// order.  In order to avoid deadlocks when holding multiple Resource / Component locks,
    /// Resources must be locked first in typeid-order, then Components in typeid-order.
    pub fn multi_lock<'a, M: Default + WorldMultiLocker<'a>>(
        &'a self,
    ) -> Result<M::Handles, Error> {
        M::default().lock(self)
    }
}

pub struct ReadResource<'a, T: 'static>(RefCell<Option<RwLockReadGuard<'a, T>>>);

impl<'a, T: 'static> Default for ReadResource<'a, T> {
    fn default() -> Self {
        ReadResource(RefCell::new(None))
    }
}

pub struct WriteResource<'a, T: 'static>(RefCell<Option<RwLockWriteGuard<'a, T>>>);

impl<'a, T: 'static> Default for WriteResource<'a, T> {
    fn default() -> Self {
        WriteResource(RefCell::new(None))
    }
}

impl<'a, T: 'static> WorldLocker<'a> for ReadResource<'a, T> {
    type Handle = RwLockReadGuard<'a, T>;

    fn id(&self) -> LockId {
        LockId::Resource(TypeId::of::<T>())
    }

    fn lock(&self, world: &'a World) -> Result<(), Error> {
        *self.0.borrow_mut() = Some(world.read_resource::<T>()?);
        Ok(())
    }

    fn handle(self) -> Self::Handle {
        self.0.into_inner().unwrap()
    }
}

impl<'a, T: 'static> WorldLocker<'a> for WriteResource<'a, T> {
    type Handle = RwLockWriteGuard<'a, T>;

    fn id(&self) -> LockId {
        LockId::Resource(TypeId::of::<T>())
    }

    fn lock(&self, world: &'a World) -> Result<(), Error> {
        *self.0.borrow_mut() = Some(world.write_resource::<T>()?);
        Ok(())
    }

    fn handle(self) -> Self::Handle {
        self.0.into_inner().unwrap()
    }
}

pub struct ReadComponent<'a, T: Component>(RefCell<Option<ComponentReadHandle<'a, T>>>);

impl<'a, T: Component> Default for ReadComponent<'a, T> {
    fn default() -> Self {
        ReadComponent(RefCell::new(None))
    }
}

pub struct WriteComponent<'a, T: Component>(RefCell<Option<ComponentWriteHandle<'a, T>>>);

impl<'a, T: Component> Default for WriteComponent<'a, T> {
    fn default() -> Self {
        WriteComponent(RefCell::new(None))
    }
}

impl<'a, T: Component> WorldLocker<'a> for ReadComponent<'a, T> {
    type Handle = ComponentReadHandle<'a, T>;

    fn id(&self) -> LockId {
        LockId::Component(TypeId::of::<T>())
    }

    fn lock(&self, world: &'a World) -> Result<(), Error> {
        *self.0.borrow_mut() = Some(world.read_component::<T>()?);
        Ok(())
    }

    fn handle(self) -> Self::Handle {
        self.0.into_inner().unwrap()
    }
}

impl<'a, T: Component> WorldLocker<'a> for WriteComponent<'a, T> {
    type Handle = ComponentWriteHandle<'a, T>;

    fn id(&self) -> LockId {
        LockId::Component(TypeId::of::<T>())
    }

    fn lock(&self, world: &'a World) -> Result<(), Error> {
        *self.0.borrow_mut() = Some(world.write_component::<T>()?);
        Ok(())
    }

    fn handle(self) -> Self::Handle {
        self.0.into_inner().unwrap()
    }
}
