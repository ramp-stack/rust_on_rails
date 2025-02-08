use super::{CanvasAppTrait, CanvasApp, CanvasContext, CanvasItem, ItemType, image};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ComponentContext {
    fonts: HashMap<String, Vec<u8>>,
    icons: HashMap<String, image::RgbaImage>
}

impl ComponentContext {
    fn get_font(&self, font: &str) -> &[u8] {
        self.fonts.get(font).unwrap()
    }

    fn get_icon(&self, icon: &str) -> &image::RgbaImage {
        self.icons.get(icon).unwrap()
    }
}

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

pub trait Drawable<'a> {
    fn draw<'b, 'c: 'b>(self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, offset: Vec2, bound: Rect, z_index: u32) -> Vec<CanvasItem<'a>>;
    fn size<'aa, 'b, 'c: 'b>(&'aa self, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>) -> Vec2;
    fn offset(&self) -> Vec2;
}

impl<'a> Drawable<'a> for ItemType<'a> {
    fn draw<'b, 'c: 'b>(self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, offset: Vec2, bound: Rect, z_index: u32) -> Vec<CanvasItem<'a>> {
        vec![CanvasItem{
            item_type: *self,
            offset: offset.into(),
            bound: bound.into(),
            z_index
        }]
    }
    fn size<'aa, 'b, 'c: 'b>(&'aa self, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>) -> Vec2 {
        let size = self.size(canvas_ctx);
        Vec2::new(size.0, size.1)
    }
    fn offset(&self) -> Vec2 {Vec2::new(0, 0)}
}

pub struct Component<'a>(Vec<Box<dyn Drawable<'a> + 'a>>, Rect);//Children, Bound, ShrinkToFit: bool, Replacing this with Transparent background Container for false

impl<'a> Drawable<'a> for Component<'a> {
    fn draw<'b, 'c: 'b>(self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, offset: Vec2, bound: Rect, z_index: u32) -> Vec<CanvasItem<'a>> {
        let offset = offset+self.1.position();
        let bound = Rect::new(
            bound.x.max(bound.x+self.1.x), bound.y.max(bound.y+self.1.y),//New bound offset
            bound.w.min(self.1.w), bound.h.min(self.1.h)//New bound size
        );
        let z_index = z_index+1;

        self.0.into_iter().flat_map(|c| c.draw(ctx, canvas_ctx, offset, bound, z_index)).collect()
    }

    //Size of an element is Max Size+Offset of its children limited to the Max size
    fn size<'aa, 'b, 'c: 'b>(&'aa self, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>) -> Vec2 {
        let size = self.0.iter().fold(Vec2::new(0, 0), |old_size, c| {
            let size = c.size(ctx, canvas_ctx);
            let offset = c.offset();
            Vec2::new(old_size.x.max(offset.x+size.x), old_size.y.max(offset.y+size.y))
        });
        Vec2::new(size.x.min(self.1.w), size.y.min(self.1.h))
    }

    fn offset(&self) -> Vec2 {self.1.position()}
}

pub trait ComponentBuilder {
    fn build_children<'a, 'b, 'c: 'b>(self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, max_size: Vec2) -> Vec<Box<dyn Drawable<'a> + 'a>>;

    fn build<'a, 'b, 'c: 'b>(self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, window: Rect) -> Component<'a> {
        Component(self.build_children(ctx, canvas_ctx, window.size()), window)
    }
}

#[derive(Clone, Copy)]
pub struct Text(pub &'static str, pub &'static str, pub &'static str, pub u32, pub u32);

impl ComponentBuilder for Text {
    fn build_children<'a, 'b, 'c: 'b>(self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, max_size: Vec2) -> Vec<Box<dyn Drawable<'a> + 'a>> {
        vec![Box::new(ItemType::Text(self.2, Some(max_size.x), self.0, ctx.get_font(self.1), self.3, self.4))]
    }
}



//  pub struct Rectangle(u32, u32, &'static str);//width, height, color

//  item_type: ItemType::Shape(Shape::Rectangle(self.0, self.1), self.2),


pub trait ComponentAppTrait {
    fn new() -> impl std::future::Future<Output = Self> where Self: Sized;
    fn root<'a, 'b, 'c, 'd>(&'a mut self, ctx: &'b ComponentContext, canvas_ctx: &'c mut CanvasContext<'d>, width: u32, height: u32) -> impl std::future::Future<Output = Box<dyn ComponentBuilder + 'static>>;
}

pub struct ComponentApp<A: ComponentAppTrait> {
    context: ComponentContext,
    app: A,
}

impl<A: ComponentAppTrait> CanvasAppTrait for ComponentApp<A> {
    async fn new() -> Self {
        //TODO: Auto import from assets
      //let prefix_path = PathBuf::from("./assets/icons");
      //let entries: Vec<_> = std::fs::read_dir(&prefix_path).unwrap()
      //.flat_map(|res| res.ok().map(|e| {
      //    let path = e.path();
      //    let icon = image::load(include_bytes!()).unwrap().to_rgba8();
      //    let path = path.strip_prefix(&prefix_path).unwrap();
      //    path.to_string_lossy().to_string()
      //})).collect();
      //println!("entries: {:?}", entries);
      //todo!();

        ComponentApp{
            context: ComponentContext{
                icons: HashMap::from([
                    ("pfp.png".to_string(), image::load_from_memory(include_bytes!("../../orangeme/assets/icons/pfp.png")).unwrap().to_rgba8()),
                    ("pfp2.png".to_string(), image::load_from_memory(include_bytes!("../../orangeme/assets/icons/pfp2.png")).unwrap().to_rgba8()),
                ]),
                fonts: HashMap::from([
                    ("outfit_bold.ttf".to_string(), include_bytes!("../../orangeme/assets/fonts/outfit_bold.ttf").to_vec()),
                    ("outfit_regular.ttf".to_string(), include_bytes!("../../orangeme/assets/fonts/outfit_regular.ttf").to_vec())
                ]),
            },
            app: A::new().await
        }
    }

    async fn draw<'a: 'b, 'b>(&'a mut self, ctx: &'b mut CanvasContext<'b>, width: u32, height: u32) -> Vec<CanvasItem<'a>> {
        let root = self.app.root(&self.context, ctx, width, height).await;
        Box::new(root.build(&self.context, ctx, Rect::new(0, 0, width, height))).draw(&self.context, ctx, Vec2::new(0, 0), Rect::new(0, 0, width, height), 0)
    }
}

#[macro_export]
macro_rules! create_entry_points {
    ($app:ty) => {
        create_winit_entry_points!(CanvasApp::<ComponentApp::<$app>>);
    };
}



pub struct Column(pub Vec<Box<dyn ComponentBuilder>>, pub u32);

impl ComponentBuilder for Column {
    fn build_children<'a, 'b, 'c: 'b>(mut self: Box<Self>, ctx: &'a ComponentContext, canvas_ctx: &'b mut CanvasContext<'c>, max_size: Vec2) -> Vec<Box<dyn Drawable<'a> + 'a>> {
        let mut bound = Rect::new(0, 0, max_size.x, max_size.y);
        self.0.into_iter().map(|builder| {
            let child = builder.build(ctx, canvas_ctx, bound);
            let height = child.size(ctx, canvas_ctx).y;
            bound.h -= height;
            bound.y += self.1 + height;
            Box::new(child) as Box<dyn Drawable>
        }).collect()
    }
}
