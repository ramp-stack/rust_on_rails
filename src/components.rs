use crate::canvas::{CanvasAppTrait, CanvasContext, CanvasItem, Area, Font, ShapeType}; // ShapeType, ItemType, image, CanvasText, ImageKey, FontKey
use crate::canvas::Text as CText;
use crate::canvas::Image as CImage;

use include_dir::{DirEntry, Dir};

use std::sync::Arc;

#[derive(Copy, Clone, Debug)]
pub struct Rect {
    pub x: u32, pub y: u32, pub w: u32, pub h: u32
}

impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Rect{x, y, w, h}
    }

    pub fn position(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub fn size(&self) -> Vec2 {
        Vec2::new(self.w, self.h)
    }
}

impl From<Rect> for (u32, u32, u32, u32) {
    fn from(rect: Rect) -> (u32, u32, u32, u32) {
        (rect.x, rect.y, rect.w, rect.h)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vec2 {
    pub x: u32,
    pub y: u32
}

impl Vec2 {
    pub fn new(x: u32, y: u32) -> Self {
        Vec2{x, y}
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self{
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl From<Vec2> for (u32, u32) {
    fn from(vec: Vec2) -> (u32, u32) {
        (vec.x, vec.y)
    }
}

pub trait Drawable {
    fn draw(&self, ctx: &mut ComponentContext, offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)>;
    fn size(&self, ctx: &mut ComponentContext) -> Vec2;
    fn offset(&self) -> Vec2;
}

impl Drawable for CanvasItem {
    fn draw(&self, _ctx: &mut ComponentContext, offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)> {
        vec![(Area(offset.into(), Some(bound.into())), self.clone())]
    }
    fn size(&self, ctx: &mut ComponentContext) -> Vec2 {
        let size = self.size(ctx);
        Vec2::new(size.x, size.y)
    }
    fn offset(&self) -> Vec2 {Vec2::new(0, 0)}
}

pub struct Component(Vec<Box<dyn Drawable>>, pub Rect);//Children, Bound, ShrinkToFit: bool, Replacing this with Transparent background Container for false

impl Drawable for Component {
    fn draw(&self, ctx: &mut ComponentContext,  offset: Vec2, bound: Rect) -> Vec<(Area, CanvasItem)> {
        let offset = offset+self.1.position();
        let bound = Rect::new(
            bound.x.max(bound.x+self.1.x), bound.y.max(bound.y+self.1.y),//New bound offset
            bound.w.min(self.1.w), bound.h.min(self.1.h)//New bound size
        );

        self.0.iter().flat_map(|c| c.draw(ctx, offset, bound)).collect()
    }

    //Size of an element is Max Size+Offset of its children limited to the Max size
    fn size(&self, ctx: &mut ComponentContext) -> Vec2 {
        let size = self.0.iter().fold(Vec2::new(0, 0), |old_size, c| {
            let size = c.size(ctx);
            let offset = c.offset();
            Vec2::new(old_size.x.max(offset.x+size.x), old_size.y.max(offset.y+size.y))
        });
        Vec2::new(size.x.min(self.1.w), size.y.min(self.1.h))
    }

    fn offset(&self) -> Vec2 {self.1.position()}
}

pub trait ComponentBuilder {
    fn build_children(&self, ctx: &mut ComponentContext, max_size: Vec2) -> Vec<Box<dyn Drawable>>;

    fn build(&self, ctx: &mut ComponentContext, window: Rect) -> Component {
        Component(self.build_children(ctx, window.size()), window)
    }

    fn on_click(&mut self, ctx: &mut ComponentContext, max_size: Vec2, position: Vec2);
    fn on_move(&mut self, ctx: &mut ComponentContext, max_size: Vec2, position: Vec2);
}

#[derive(Clone)]
pub struct Text(pub &'static str, pub &'static str, pub u8, pub Option<u32>, pub u32, pub u32, pub Font);
// Text, Color, Opacity, Optional Width, text size, line height, font

impl ComponentBuilder for Text {
    fn build_children(&self, _ctx: &mut ComponentContext, max_size: Vec2) -> Vec<Box<dyn Drawable>> {
        vec![Box::new(CanvasItem::Text(CText::new(self.0, self.1, self.2, self.3, self.4, self.5, self.6.clone())))]
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}

    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}
}

#[derive(Clone)]
pub struct Shape(pub ShapeType, pub &'static str, pub u8);
//  Shape, color, opacity

impl ComponentBuilder for Shape {
    fn build_children(&self, _ctx: &mut ComponentContext, _max_size: Vec2) -> Vec<Box<dyn Drawable>> {
        vec![Box::new(CanvasItem::Shape(self.0, self.1, self.2))]
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}

    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}
}

#[derive(Clone)]
pub struct Image(pub ShapeType, pub CImage);
// Shape, Image

impl ComponentBuilder for Image {
    fn build_children(&self, _ctx: &mut ComponentContext, _max_size: Vec2) -> Vec<Box<dyn Drawable>> {
        vec![Box::new(CanvasItem::Image(self.0, self.1.clone()))]
    }

    fn on_click(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}

    fn on_move(&mut self, _ctx: &mut ComponentContext, _max_size: Vec2, _position: Vec2) {}
}


// #[derive(Clone)]
// pub struct Handle{
//     font: Option<Arc<FontKey>>,
//     image: Option<Arc<ImageKey>>,
// }

// impl Handle {
//     pub fn u(&self) -> u64 {
//         **self.font.as_ref().unwrap_or_else(|| self.image.as_ref().unwrap())
//     }

//     fn new_font(key: FontKey) -> Self {
//         Handle{font: Some(Arc::new(key)), image: None}
//     }

//     fn new_image(key: ImageKey) -> Self {
//         Handle{font: None, image: Some(Arc::new(key))}
//     }

//     fn try_drop(self, ctx: &mut CanvasContext) -> Option<Self> {
//         if let Some(font) = self.font {
//             match Arc::try_unwrap(font) {
//                 Ok(f) => {
//                     ctx.atlas.remove_font(&f);
//                     None
//                 },
//                 Err(e) => Some(Handle{font: Some(e), image: None})
//             }
//         } else if let Some(image) = self.image {
//             match Arc::try_unwrap(image) {
//                 Ok(i) => {
//                     ctx.atlas.remove_image(&i);
//                     None
//                 },
//                 Err(e) => Some(Handle{font: None, image: Some(e)})
//             }
//         } else {None}
//     }
// }

pub struct ComponentContext<'a> {
    // handles: &'a mut Vec<Handle>,
    assets: &'a mut Vec<Dir<'static>>,
    canvas: &'a mut CanvasContext
}

impl<'a> ComponentContext<'a> {
    pub fn new(assets: &'a mut Vec<Dir<'static>>, canvas: &'a mut CanvasContext) -> Self {
        ComponentContext{assets, canvas}
    }

    pub fn include_assets(&mut self, dir: Dir<'static>) {
        self.assets.push(dir);
    }

    pub fn load_file(&self, file: &'static str) -> Option<Vec<u8>> {
        self.assets.iter().find_map(|dir|
            dir.find(file).ok().and_then(|mut f|
                f.next().and_then(|f|
                    if let DirEntry::File(f) = f {Some(f.contents().to_vec())} else {None}
                )
            )
        )
    }

    pub fn load_font(&mut self, font: &'static str) -> Option<Font> {
        self.load_file(font).map(|font| self.add_font(font))
    }

    pub fn load_image(&mut self, image: &'static str) -> Option<CImage> {
        self.load_file(image).map(|image| {
            self.add_image(image::load_from_memory(&image).unwrap().to_rgba8())
        })
    }

    pub fn add_image(&mut self, image: image::RgbaImage) -> CImage {
        self.canvas.new_image(image)
    }

    pub fn add_font(&mut self, font: Vec<u8>) -> Font {
        self.canvas.new_font(font)
    }

    pub fn measure_text(&mut self, text: &wgpu_canvas::Text) -> (u32, u32) {
        text.size(&mut self.canvas.atlas)
    }
}

pub trait ComponentAppTrait {
    fn new(ctx: &mut ComponentContext) -> impl std::future::Future<Output = Box<dyn ComponentBuilder>> where Self: Sized;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    // handles: Option<Vec<Handle>>,
    assets: Vec<Dir<'static>>,
    app: Box<dyn ComponentBuilder>,
    _p: std::marker::PhantomData<A>
}

impl<A: ComponentAppTrait> ComponentApp<A> {
    // fn trim(&mut self, ctx: &mut CanvasContext) {
    //     self.handles = Some(self.handles.take().unwrap().into_iter().flat_map(|h| {
    //         h.try_drop(ctx)
    //     }).collect());
    // }
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    async fn new(ctx: &mut CanvasContext) -> Self {
        // let mut handles = Some(Vec::new());
        let mut assets = Vec::new();
        let mut ctx = ComponentContext::new(&mut assets, ctx);
        let app = A::new(&mut ctx).await;
        ComponentApp{assets, app, _p: std::marker::PhantomData::<A>}
    }

    async fn on_tick(&mut self, ctx: &mut CanvasContext) {
        // self.trim(ctx);
        let width = ctx.size.logical().0;
        let height = ctx.size.logical().1;
        let mut ctx = ComponentContext::new(&mut self.assets, ctx);
        self.app.build(&mut ctx, Rect::new(0, 0, width, height))
            .draw(&mut ctx, Vec2::new(0, 0), Rect::new(0, 0, width, height))
            .into_iter().for_each(|(i, a)| ctx.canvas.draw(i, a))
    }

    async fn on_click(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.size.logical().0;
        let height = ctx.size.logical().1;
        let x = ctx.position.0;
        let y = ctx.position.1;
        let mut ctx = ComponentContext::new(&mut self.assets, ctx);
        self.app.on_click(&mut ctx, Vec2::new(width, height), Vec2::new(x, y));
    }

    async fn on_move(&mut self, ctx: &mut CanvasContext) {
        let width = ctx.size.logical().0;
        let height = ctx.size.logical().1;
        let x = ctx.position.0;
        let y = ctx.position.1;
        let mut ctx = ComponentContext::new(&mut self.assets, ctx);
        self.app.on_move(&mut ctx, Vec2::new(width, height), Vec2::new(x, y));
    }

    async fn on_press(&mut self, _ctx: &mut CanvasContext, _t: String) {
      //let mut ctx = ComponentContext::new(self.handles.as_mut().unwrap(), &mut self.assets, ctx);
      //self.app.on_press(&mut ctx, Vec2::new(width, height), t);
    }
}

#[macro_export]
macro_rules! create_component_entry_points {
    ($app:ty) => {
        create_canvas_entry_points!(ComponentApp::<$app>);
    };
}
