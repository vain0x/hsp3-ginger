use helpers;
use hspsdk;
use std::slice;

pub(crate) type TyFlag = i32;
pub(crate) type Aptr = i32;
pub(crate) type DataPtr = *mut *mut std::ffi::c_void;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum Ty {
    Str,
    Double,
    Int,
    Unknown(TyFlag),
}

impl Ty {
    pub fn from_flag(flag: TyFlag) -> Self {
        match flag as u32 {
            hspsdk::HSPVAR_FLAG_STR => Ty::Str,
            hspsdk::HSPVAR_FLAG_DOUBLE => Ty::Double,
            hspsdk::HSPVAR_FLAG_INT => Ty::Int,
            _ => Ty::Unknown(flag),
        }
    }

    pub fn to_flag(self) -> TyFlag {
        match self {
            Ty::Str => hspsdk::HSPVAR_FLAG_STR as TyFlag,
            Ty::Double => hspsdk::HSPVAR_FLAG_DOUBLE as TyFlag,
            Ty::Int => hspsdk::HSPVAR_FLAG_INT as TyFlag,
            Ty::Unknown(flag) => flag,
        }
    }

    // FIXME: HspVarProc::name
    pub fn name(self) -> &'static str {
        match self {
            Ty::Str => "str",
            Ty::Double => "double",
            Ty::Int => "int",
            Ty::Unknown(_) => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ValueRef<'a> {
    ty: Ty,
    data_ptr: DataPtr,
    lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> ValueRef<'a> {
    pub fn new(ty: Ty, data_ptr: DataPtr) -> Self {
        ValueRef {
            ty,
            data_ptr,
            lifetime: std::marker::PhantomData,
        }
    }

    pub fn to_copy(&self) -> ValueCopy {
        match self.ty {
            Ty::Str => ValueCopy::Str(helpers::string_from_hsp_str(self.data_ptr as *const u8)),
            Ty::Double => ValueCopy::Double(unsafe { *(self.data_ptr as *const f64) }),
            Ty::Int => ValueCopy::Int(unsafe { *(self.data_ptr as *const i32) }),
            _ => ValueCopy::Unknown(self.ty),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ValueCopy {
    Str(String),
    Double(f64),
    Int(i32),
    Unknown(Ty),
}

impl ValueCopy {
    pub fn into_string(self) -> String {
        match self {
            ValueCopy::Str(value) => value,
            ValueCopy::Double(value) => value.to_string(),
            ValueCopy::Int(value) => value.to_string(),
            ValueCopy::Unknown(_) => "<unknown-value>".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct HspContext<T> {
    inner: T,
}

impl<'s, T> HspContext<T>
where
    T: std::borrow::Borrow<hspsdk::HSPCTX> + 's,
{
    pub fn from(inner: T) -> Self {
        HspContext { inner }
    }

    fn inner<'a: 's>(&'a self) -> &'s hspsdk::HSPCTX {
        self.inner.borrow()
    }

    fn header(&self) -> &'s hspsdk::HSPHED {
        unsafe { &*self.inner().hsphed }
    }

    fn var_proc(&self, flag: TyFlag) -> &'s hspsdk::HspVarProc {
        let get_proc = self.inner().exinfo.HspFunc_getproc.unwrap();
        unsafe { &*get_proc(flag) }
    }

    pub fn var_element_ref<'a>(&self, pval: &'a mut hspsdk::PVal, aptr: Aptr) -> ValueRef<'a> {
        let get_ptr = self.var_proc(pval.flag as TyFlag).GetPtr.unwrap();
        pval.offset = aptr;
        let data_ptr = unsafe { get_ptr(pval) };
        ValueRef::new(Ty::from_flag(pval.flag as TyFlag), data_ptr)
    }
}

impl<'s, T> HspContext<T>
where
    T: std::borrow::BorrowMut<hspsdk::HSPCTX> + 's,
{
    fn inner_mut<'a: 's>(&'a mut self) -> &'s mut hspsdk::HSPCTX {
        self.inner.borrow_mut()
    }

    pub fn static_vars(&mut self) -> &'s mut [hspsdk::PVal] {
        let var_count = self.header().max_val as usize;
        let pvals = unsafe { slice::from_raw_parts_mut(self.inner().mem_var, var_count) };
        pvals
    }
}
