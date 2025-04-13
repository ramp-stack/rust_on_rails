pub struct LocalFuture<T>(Arc<Mutex<Option<T>>>);

impl<T> LocalFuture<T> {
    pub fn unwrap(self) -> T {
        Arc::into_inner(self.lock).unwrap().into_inner().unwrap().expect("Future did not complete")
    }
}

pub fn spawn_local<T: 'static>(&self, task: impl Future<Output = T> + 'static) -> LocalFuture<T> {
    let lock = Arc::new(Mutex::new(None));
    let local_future = LocalFuture{lock: lock.clone()};
    let future = async move { *lock.lock().unwrap() = Some(task.await); };

    wasm_bindgen_futures::spawn_local(future);

    local_future
}



//  #[macro_export]
//  macro_rules! create_base_entry_points {
//      ($app:ty, $bg_app:ty) => {
//          #[cfg(target_arch = "wasm32")]
//          #[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
//          pub fn wasm_main() {
//              todo!()//TODO: Create a wasm base app in another file that has no runtime
//              //WinitApp::new(BaseApp::<$app>::new(env!("CARGO_PKG_NAME"))).start()
//          }

//          #[cfg(target_os = "android")]
//          #[no_mangle]
//          fn android_main(app: AndroidApp) {
//              WinitApp::new(BaseApp::<$app>::new(env!("CARGO_PKG_NAME"))).start(app)
//          }

//          #[cfg(target_os = "ios")]
//          #[no_mangle]
//          pub extern "C" fn ios_main() {
//              WinitApp::new(BaseApp::<$app>::new(env!("CARGO_PKG_NAME"))).start()
//          }

//          #[cfg(target_os="linux")]
//          pub fn desktop_main() {
//              if std::env::args().collect::<Vec<_>>().len() > 1 {
//                  BaseBGApp::<$bg_app>::new(env!("CARGO_PKG_NAME"))
//              } else {
//                  WinitApp::new(BaseApp::<$app>::new(env!("CARGO_PKG_NAME"))).start()
//              }
//          }

//          #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "linux", target_arch = "wasm32")))]
//          pub fn desktop_main() {
//              WinitApp::new(BaseApp::<$app>::new(env!("CARGO_PKG_NAME"))).start()
//          }

//          #[cfg(target_os = "ios")]
//          #[no_mangle]
//          pub extern "C" fn ios_background() {
//              BaseBGApp::<$bg_app>::new(env!("CARGO_PKG_NAME"))
//          }
//      };
//  }
