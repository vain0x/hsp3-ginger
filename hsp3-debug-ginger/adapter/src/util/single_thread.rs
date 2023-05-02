use std::thread::{self, ThreadId};

/// オブジェクトを単一のスレッドからのみアクセスできるようにガードする
///
/// - この型はすべての `T` に対して Send, Sync トレイトを実装する。次のような状況への解決策として利用できる
///     - T が Send/Sync を実装するように要求されているが、T が Send/Sync を実装していない
///     - T は実際には単一のスレッドからしかアクセスされない
pub struct SingleThread<T> {
    /// このオブジェクトが構築されたスレッドのID
    thread_id: ThreadId,
    value: T,
}

impl<T> SingleThread<T> {
    /// 値をラップする
    ///
    /// この関数が生成するオブジェクトは、この関数が呼ばれたスレッドを記憶する。
    /// このスレッドからのアクセスだけを許可する
    pub fn new(value: T) -> Self {
        SingleThread {
            thread_id: thread::current().id(),
            value,
        }
    }

    /// 値への共有参照を取得する
    ///
    /// この関数は元のスレッドからのアクセスのみ許可する。そうでなければパニックを起こす
    #[allow(unused)]
    pub fn get(&self) -> &T {
        assert_eq!(self.thread_id, thread::current().id());
        &self.value
    }

    /// 値への排他参照を取得する
    ///
    /// この関数は元のスレッドからのアクセスのみ許可する。そうでなければパニックを起こす
    #[allow(unused)]
    pub fn get_mut(&mut self) -> &mut T {
        assert_eq!(self.thread_id, thread::current().id());
        &mut self.value
    }

    /// もとの値を取り出す
    ///
    /// この関数は元のスレッドからのアクセスのみ許可する。そうでなければパニックを起こす
    #[allow(unused)]
    pub fn into_inner(self) -> T {
        assert_eq!(self.thread_id, thread::current().id());
        self.value
    }
}

unsafe impl<T> Send for SingleThread<T> {}
unsafe impl<T> Sync for SingleThread<T> {}
