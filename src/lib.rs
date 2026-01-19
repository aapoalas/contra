use std::{collections::HashSet, marker::PhantomData};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Covariant<'a>(PhantomData<fn() -> &'a ()>);
impl Covariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Contravariant<'a>(PhantomData<fn(&'a ())>);
impl Contravariant<'_> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a> Clone for Contravariant<'a>
where
    Self: 'a,
{
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<'a> Copy for Contravariant<'a> {}

impl Default for Contravariant<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum ArenaHandle<'a> {
    Value(f64),
    Ref(Handle<'a>),
}

#[derive(Debug)]
pub struct Arena<'a> {
    data: Vec<ArenaHandle<'a>>,
    roots: Vec<Option<Handle<'a>>>,
}

impl<'a> Arena<'a> {
    pub fn new(capacity: usize) -> (Self, Gc<'a>) {
        let mut arena = Self {
            data: Vec::with_capacity(capacity),
            roots: Vec::with_capacity(8),
        };
        let root = arena.alloc(0.0);
        arena.roots.push(Some(root));
        // SAFETY: created with Arena.
        let gc = unsafe { Gc::new() };
        (arena, gc)
    }

    pub fn alloc<'l>(&mut self, value: f64) -> Handle<'l>
    where
        'a: 'l,
    {
        let index = u32::try_from(self.data.len()).unwrap();
        self.data.push(ArenaHandle::Value(value));
        Handle::new(index)
    }

    pub fn store<'h>(&mut self, handle: Handle<'h>) -> Handle<'h>
    where
        'a: 'h,
    {
        let index = u32::try_from(self.data.len()).unwrap();
        self.data.push(ArenaHandle::Ref(handle));
        Handle::new(index)
    }

    pub fn gc<'gc>(&mut self, _gc: Gc<'gc>)
    where
        'a: 'gc,
    {
        let mut kept_handles = HashSet::with_capacity(self.data.len());
        self.roots.iter().for_each(|handle| {
            // SAFETY: we promise to keep these handles valid for this time.
            let mut handle = handle.as_ref().map(|h| unsafe { h.copy() });
            while let Some(h) = handle {
                let result = self.data.get(h.0 as usize).and_then(|d| match d {
                    ArenaHandle::Value(_) => None,
                    ArenaHandle::Ref(handle) => Some(unsafe { handle.copy() }),
                });
                // SAFETY: we promise to keep these handles valid for this time.
                kept_handles.insert(h);
                handle = result;
            }
        });
        let mut new_handles = kept_handles
            .iter()
            // SAFETY: we promise to keep these handles valid for this time.
            .map(|h| unsafe { h.copy() })
            .collect::<Vec<_>>();
        let mut prev = 0;
        while kept_handles.len() > prev {
            prev = kept_handles.len();
            new_handles.sort();
            for handle in new_handles.iter() {
                match handle.get(self) {
                    // SAFETY: we promise to keep these handles valid for this time.
                    ArenaHandle::Ref(d) => {
                        kept_handles.insert(unsafe { d.copy() });
                    }
                    _ => {}
                }
            }
            new_handles.clear();
        }
        let mut kept_handles = kept_handles.into_iter().enumerate().collect::<Vec<_>>();
        kept_handles.sort();
        eprintln!("Kept handles: {kept_handles:?}");
        eprintln!("Data: {:?}", self.data);
        let mut idx = 0u32;
        self.data.retain_mut(|handle| {
            let i = idx;
            idx += 1;
            if kept_handles
                .binary_search_by_key(&i, |h| h.0 as u32)
                .is_err()
            {
                return false;
            }
            if let ArenaHandle::Ref(target) = handle {
                let target_index = kept_handles
                    .binary_search_by_key(&target.0, |h| h.0 as u32)
                    .unwrap() as u32;
                *target = Handle::new(target_index);
            }
            true
        });
        self.roots.iter_mut().for_each(|handle| {
            if let Some(target) = handle {
                eprintln!("Target handle: {target:?}");
                let target_index = kept_handles
                    .binary_search_by_key(&target.0, |h| h.0 as u32)
                    .unwrap() as u32;
                *target = Handle::new(target_index);
            }
        });
        eprintln!("Data: {:?}", self.data);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Handle<'a>(u32, Contravariant<'a>);

impl<'a> Handle<'a> {
    fn new(index: u32) -> Handle<'a> {
        Handle(index, Contravariant::new())
    }

    pub fn scope<'ar>(self, arena: &mut Arena<'ar>) -> Global<'ar>
    where
        'ar: 'a,
    {
        let index = arena.roots.len();
        arena.roots.push(Some(self));
        Global {
            index: index as u32,
            _marker: Contravariant::new(),
        }
    }

    pub fn set_value(&self, arena: &mut Arena<'a>, value: f64) {
        let contents = self.get_mut(arena);
        *contents = ArenaHandle::Value(value)
    }

    pub fn set_handle(&self, arena: &mut Arena<'a>, handle: Handle<'a>) {
        let contents = self.get_mut(arena);
        *contents = ArenaHandle::Ref(handle)
    }

    // pub fn internal_set(
    //     self,
    //     arena: &mut Arena,
    //     p: Self,
    //     v: Self,
    //     o: Self,
    //     gc: Gc<'a>,
    // ) -> Result<bool, Self> {
    //     Ok(true)
    // }

    /// Test function to "safely" test compile-time invalidation of Handles.
    pub fn test_usage(&self) {}

    pub fn get_value<'arena>(&self, arena: &'arena Arena<'a>) -> &'arena f64 {
        eprintln!("index: {}", self.0);
        match arena.data.get(self.0 as usize).unwrap() {
            ArenaHandle::Value(v) => v,
            ArenaHandle::Ref(handle) => handle.get_value(arena),
        }
    }

    pub(crate) fn get<'arena>(&self, arena: &'arena Arena<'a>) -> &'arena ArenaHandle<'a> {
        arena.data.get(self.0 as usize).unwrap()
    }

    pub(crate) fn get_mut<'arena>(
        &self,
        arena: &'arena mut Arena<'a>,
    ) -> &'arena mut ArenaHandle<'a> {
        arena.data.get_mut(self.0 as usize).unwrap()
    }

    pub unsafe fn local<'l>(self) -> Handle<'l>
    where
        'a: 'l,
    {
        Handle(self.0, Contravariant::new())
    }

    pub unsafe fn copy<'l>(&self) -> Handle<'l>
    where
        'a: 'l,
    {
        Handle(self.0, Contravariant::new())
    }
}

pub struct Global<'a> {
    index: u32,
    _marker: Contravariant<'a>,
}

impl<'a> Global<'a> {
    pub fn get(&self, arena: &Arena<'a>) -> Handle<'a> {
        // SAFETY: caller's problem, not mine.
        unsafe { arena.roots[self.index as usize].as_ref().unwrap().copy() }
    }

    pub fn take<'l>(self, arena: &mut Arena) -> Handle<'l>
    where
        'a: 'l,
    {
        // SAFETY: caller's problem, not mine.
        unsafe { arena.roots[self.index as usize].take().unwrap().local() }
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

    #[inline(always)]
    pub fn join<'a>(&'a self, _: &Handle<'a>) {}

    pub fn reborrow(&mut self) -> Gc<'_> {
        Gc(Covariant::new())
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
    #[inline(always)]
    pub fn join(self, _: Handle<'a>) {}
}

#[macro_export]
macro_rules! bind {
    ($handle: ident, $gc: expr) => {
        // SAFETY: immediately joined to Gc.
        let $handle = unsafe { $handle.local() };
        $gc.join(&$handle);
    };
    (let $handle: ident = $handle_creation: expr, $gc: expr) => {
        let $handle = $handle_creation;
        $gc.join(&$handle);
    };
}

#[cfg(test)]
mod tests {
    use crate::{Arena, Gc, Handle, bind};

    #[test]
    fn test_invalidation() {
        fn inner<'a: 'gc, 'gc>(
            arena: &mut Arena<'a>,
            handle: Handle<'gc>,
            gc: Gc<'gc>,
        ) -> Handle<'gc> {
            bind!(handle, gc);
            bind!(let handle = arena.store(handle), gc);
            bind!(let handle = arena.store(handle), gc);
            bind!(let handle = arena.alloc(0.123456), gc);
            bind!(let handle = arena.store(handle), gc);
            bind!(let handle = arena.store(handle), gc);
            bind!(let handle = arena.store(handle), gc);
            handle
        }

        fn outer<'a: 'gc, 'gc>(
            arena: &mut Arena<'a>,
            handle: Handle<'gc>,
            mut gc: Gc<'gc>,
        ) -> Handle<'gc> {
            bind!(handle, gc);
            handle.test_usage();
            bind!(let result = inner(arena, handle, gc.reborrow()), gc);
            bind!(let storage = arena.store(result), gc);
            let bad_result = result;
            eprintln!("Stored result: {}", storage.get_value(arena));
            let storage = storage.scope(arena);
            eprintln!("Stored result: {}", storage.get(arena).get_value(arena));
            arena.gc(gc.reborrow());
            bind!(let storage = storage.take(arena), gc);
            eprintln!("Stored result: {}", storage.get_value(arena));
            bad_result
        }

        let (mut arena, gc) = Arena::new(256);
        bind!(let handle = arena.alloc(1.235), gc);
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
