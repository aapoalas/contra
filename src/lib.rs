use std::marker::PhantomData;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Covariant<'a>(PhantomData<fn() -> &'a ()>);
impl Covariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct Contravariant<'a>(PhantomData<fn(&'a ())>);
impl Contravariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl Default for Contravariant<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Arena(Vec<f64>);

impl Arena {
    pub fn new<'a>(capacity: usize) -> (Self, Gc<'a>) {
        // SAFETY: created with Arena.
        (Self(Vec::with_capacity(capacity)), unsafe { Gc::new() })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Handle<'a>(u32, Contravariant<'a>);

impl<'a> Handle<'a> {
    pub fn new(arena: &mut Arena, value: f64) -> Handle<'a> {
        let index = u32::try_from(arena.0.len()).unwrap();
        arena.0.push(value);
        Handle(index, Contravariant::new())
    }

    pub fn get<'arena>(self, arena: &'arena Arena, _gc: NoGc<'a>) -> &'arena f64 {
        arena.0.get(self.0 as usize).as_ref().unwrap()
    }

    pub fn local<'l>(self) -> Handle<'l> {
        Handle(self.0, Contravariant::new())
    }

    pub fn unbind(self) -> Handle<'static> {
        Handle(self.0, Contravariant::new())
    }
}

#[derive(Debug)]
pub struct Gc<'a>(Covariant<'a>);

impl<'gc> Gc<'gc> {
    /// # Safety
    ///
    /// Only one Gc must exist per
    unsafe fn new() -> Self {
        Gc(Covariant::new())
    }

    pub fn join(&'gc self, _: Handle<'gc>) {}

    pub fn reborrow(&mut self) -> Gc<'_> {
        Gc(Covariant::new())
    }

    pub fn nogc(&self) -> NoGc<'_> {
        NoGc(Covariant::new())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoGc<'a>(Covariant<'a>);

#[cfg(test)]
mod tests {
    use crate::{Arena, Gc, Handle};

    #[test]
    fn test_invalidation() {
        fn perform_gc<'a>(arena: &mut Arena, _: Gc<'a>) {
            arena.0.clear();
        }

        fn inner<'gc>(
            arena: &mut Arena,
            a: Handle,
            b: Handle,
            mut gc: Gc<'gc>,
        ) -> Result<Handle<'gc>, Handle<'gc>> {
            let a = a.local();
            let b = b.local();
            gc.join(a);
            // gc.join(b);
            a.get(arena, gc.nogc());
            perform_gc(arena, gc.reborrow());
            // a.get(arena, gc.nogc());
            // b.get(arena);
            Err(Handle::new(arena, 1.235))
        }

        fn outer<'gc>(
            arena: &mut Arena,
            handle: Handle,
            mut gc: Gc<'gc>,
        ) -> Result<Handle<'gc>, Handle<'gc>> {
            let handle = handle.local();
            gc.join(handle);
            let result = inner(arena, handle, handle, gc.reborrow())?;
            // handle.get(arena);
            // gc.join(result);
            let scoped_result = *result.get(arena, gc.nogc());
            perform_gc(arena, gc.reborrow());
            // result.get(arena, gc.nogc());
            Ok(Handle::new(arena, scoped_result))
        }

        let (mut arena, gc) = Arena::new(256);
        let handle = Handle::new(&mut arena, 1.0);
        gc.join(handle);
        outer(&mut arena, handle, gc).unwrap_err();
    }
}
