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
    pub fn new(arena: &mut Arena, value: f64, gc: Gc<'a>) -> Handle<'a> {
        let index = u32::try_from(arena.0.len()).unwrap();
        arena.0.push(value);
        Handle(index, Contravariant::new())
    }

    pub fn scope(self, arena: &mut Arena, gc: NoGc<'a>) -> Scoped {
        let data = *self.get(arena, gc);
        Scoped { data }
    }

    pub fn internal_set(
        self,
        arena: &mut Arena,
        p: Self,
        v: Self,
        o: Self,
        gc: Gc<'a>,
    ) -> Result<bool, Self> {
        Ok(true)
    }

    pub fn asd(self, _gc: NoGc<'a>) -> u32 {
        0
    }

    pub fn thing(self) -> u32 {
        0
    }

    pub fn get<'arena>(self, arena: &'arena Arena, _gc: NoGc<'a>) -> &'arena f64 {
        arena.0.get(self.0 as usize).as_ref().unwrap()
    }

    pub fn local<'l>(self) -> Handle<'l>
    where
        'a: 'l,
    {
        Handle(self.0, Contravariant::new())
    }

    pub fn unbind(self) -> Handle<'static> {
        Handle(self.0, Contravariant::new())
    }
}

pub struct Scoped {
    data: f64,
}

impl Scoped {
    pub fn get<'a>(&self, arena: &mut Arena) -> Handle<'a> {
        todo!()
        // Handle::new(arena, self.data)
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

    pub fn join<'a>(&'a self, _: Handle<'a>) {}

    pub fn reborrow(&mut self) -> Gc<'_> {
        Gc(Covariant::new())
    }

    pub fn nogc(&self) -> NoGc<'_> {
        NoGc(Covariant::new())
    }

    pub fn into_nogc(self) -> NoGc<'gc> {
        self.into()
    }
}

impl<'a> From<Gc<'a>> for NoGc<'a> {
    fn from(_: Gc<'a>) -> Self {
        NoGc(Covariant::new())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoGc<'a>(Covariant<'a>);

impl<'a> NoGc<'a> {
    pub fn join(self, _: Handle<'a>) {}
}

#[cfg(test)]
mod tests {
    use crate::{Arena, Gc, Handle};

    #[test]
    fn test_invalidation() {
        fn invalidate<'a>(_: Gc<'a>, handle: Handle) -> &'a () {
            todo!();
        }
        fn invalidate2<'a>(handle: Handle<'a>, _: Gc<'a>) {}

        fn inner<'gc>(arena: &mut Arena, handle: Handle<'_>, gc: Gc<'gc>) {}

        fn outer<'gc>(arena: &mut Arena, handle: Handle<'gc>, mut gc: Gc<'gc>) {
            let bad_handle: Handle<'gc> = handle;
            let handle = handle.local();
            gc.join(handle);
            let result = invalidate2(bad_handle, gc.reborrow());
            bad_handle.thing();
            println!("result");
        }

        let (mut arena, mut gc) = Arena::new(256);
        let handle = Handle::new(&mut arena, 1.0, gc.reborrow());
        outer(&mut arena, handle, gc);
    }

    // #[test]
    // fn test_set() {
    //     use crate::{
    //         Arena as Agent, Gc as GcScope, Handle as Object, Handle as PropertyKey,
    //         Handle as Value, Handle as JsResult, Handle as JsError, NoGc as NoGcScope,
    //     };

    //     pub(crate) fn throw_set_error<'a>(
    //         agent: &mut Agent,
    //         p: PropertyKey<'a>,
    //         gc: NoGcScope<'a>,
    //     ) -> JsError<'a> {
    //         p
    //     }

    //     pub(crate) fn set<'a>(
    //         agent: &mut Agent,
    //         o: Object,
    //         p: PropertyKey,
    //         v: Value,
    //         throw: bool,
    //         mut gc: GcScope<'a>,
    //     ) -> Result<(), JsError<'a>> {
    //         let nogc = gc.nogc();
    //         let o = o.local();
    //         nogc.join(o);
    //         let p = p.local();
    //         nogc.join(p);
    //         let v = v.local();
    //         nogc.join(v);
    //         let scoped_p = p.scope(agent, nogc);
    //         let success = o.internal_set(agent, p, v, o.into(), gc.reborrow())?;
    //         let p = scoped_p.get(agent);
    //         gc.join(p);
    //         if !success && throw {
    //             return Err(throw_set_error(agent, p, gc.into_nogc()));
    //         }
    //         Ok(())
    //     }
    // }
}
