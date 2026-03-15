use winit::event_loop::{ActiveEventLoop};
use winit::window::{Window, WindowAttributes};
use winit::event::WindowEvent;
use winit::application::ApplicationHandler;

#[derive(Default)]
pub struct App {
    window: Option<Box<dyn Window>>
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("rust-renderer");

        let window = event_loop.create_window(attrs).unwrap();

        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            _ => ()
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.window = Some(event_loop.create_window(WindowAttributes::default()).unwrap());
    }
}
