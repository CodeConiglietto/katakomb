use ggez::{
    graphics::{spritebatch::SpriteBatch, BlendMode, DrawParam, Drawable, Image, Rect},
    Context, GameResult,
};

pub struct KataFont {
    pub texture: Image,
    char_width: u8,
    char_height: u8,
}

pub fn load_font(ctx: &mut Context) -> KataFont {
    let texture = image::open(r"C:\Users\admin\Documents\katakomb\resources\master8x8.png")
        .unwrap()
        .to_rgba();

    KataFont {
        texture: Image::from_rgba8(
            ctx,
            texture.width() as u16,
            texture.height() as u16,
            &texture.into_raw(),
        )
        .unwrap(),
        char_width: 8,
        char_height: 8,
    }
}

pub fn get_font_offset(index: u16, font: &KataFont) -> Rect {
    let font_width = font.texture.width();
    let font_height = font.texture.height();
    let float_char_width = font.char_width as f32 / font_width as f32;
    let float_char_height = font.char_height as f32 / font_height as f32;

    let chars_width = 16;
    // let chars_height = 64;

    let x_index = index % chars_width;
    let y_index = index / chars_width;

    Rect::new(
        x_index as f32 * float_char_width,
        y_index as f32 * float_char_height,
        float_char_width,
        float_char_height,
    )
}

pub struct KataFontBatch {
    font: KataFont,
    sprite_batch: SpriteBatch,
}

impl KataFontBatch {
    pub fn new(font: KataFont) -> Self {
        Self {
            sprite_batch: SpriteBatch::new(font.texture.clone()),
            font,
        }
    }

    /*
    pub fn add<P>(voxel_face: VoxelFace) -> SpriteIdx
    where
        P: Into<DrawParam>,
    {
    }
    */
}

impl Drawable for KataFontBatch {
    fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.sprite_batch.draw(ctx, param)
    }

    fn dimensions(&self, ctx: &mut Context) -> Option<Rect> {
        self.sprite_batch.dimensions(ctx)
    }

    fn set_blend_mode(&mut self, mode: Option<BlendMode>) {
        self.sprite_batch.set_blend_mode(mode)
    }

    fn blend_mode(&self) -> Option<BlendMode> {
        self.sprite_batch.blend_mode()
    }
}
