use failure::Fallible;
use ggez::{
    graphics::{spritebatch::SpriteBatch, BlendMode, DrawParam, Drawable, FilterMode, Image, Rect},
    Context, GameResult, mint,
};

use crate::rendering::voxel::Voxel2;

pub struct KataFont {
    texture: Image,
    char_width: u8,
    char_height: u8,
}

impl KataFont {
    pub fn texture(&self) -> &Image {
        &self.texture
    }

    pub fn load(ctx: &mut Context) -> Fallible<Self> {
        let texture = Image::new(ctx, "/master8x8.png")?;

        let char_width: u8 = 8;
        let char_height: u8 = 8;

        assert_eq!(
            texture.width() % u16::from(char_width),
            0,
            "Font width {} is not multiple of char width {}",
            texture.width(),
            char_width,
        );

        assert_eq!(
            texture.height() % u16::from(char_height),
            0,
            "Font height {} is not multiple of char height {}",
            texture.height(),
            char_height,
        );

        Ok(Self {
            texture,
            char_width,
            char_height,
        })
    }

    pub fn get_src_rect(&self, index: u16) -> Rect {
        let font_width = self.texture.width();
        let font_height = self.texture.height();
        let float_char_width = self.char_width as f32 / font_width as f32;
        let float_char_height = self.char_height as f32 / font_height as f32;

        let charset_width = self.charset_width();

        let x_index = index % charset_width;
        let y_index = index / charset_width;

        Rect::new(
            x_index as f32 * float_char_width,
            y_index as f32 * float_char_height,
            float_char_width,
            float_char_height,
        )
    }

    pub fn char_width(&self) -> u8 {
        self.char_width
    }

    pub fn char_height(&self) -> u8 {
        self.char_height
    }

    pub fn charset_width(&self) -> u16 {
        self.texture.width() / u16::from(self.char_width)
    }

    pub fn charset_height(&self) -> u16 {
        self.texture.height() / u16::from(self.char_height)
    }
}

pub struct KataFontBatch {
    font: KataFont,
    fg_batch: SpriteBatch,
    bg_batch: SpriteBatch,
    scaling: f32,
}

impl KataFontBatch {
    pub fn new(font: KataFont, white_image: Image, scaling: f32) -> Self {
        let mut fg_batch = SpriteBatch::new(font.texture.clone());
        let mut bg_batch = SpriteBatch::new(white_image);

        fg_batch.set_filter(FilterMode::Nearest);
        bg_batch.set_filter(FilterMode::Nearest);

        fg_batch.set_blend_mode(Some(BlendMode::Alpha));
        bg_batch.set_blend_mode(Some(BlendMode::Alpha));

        Self {
            fg_batch,
            bg_batch,
            font,
            scaling,
        }
    }

    pub fn set_scaling(&mut self, scaling: f32) {
        self.scaling = scaling;
    }

    pub fn scaling(&self) -> f32 {
        self.scaling
    }

    pub fn tile_width(&self) -> f32 {
        self.scaling * self.font.char_width() as f32
    }

    pub fn tile_height(&self) -> f32 {
        self.scaling * self.font.char_height() as f32
    }

    pub fn add<P>(&mut self, voxel: &Voxel2, dest: P)
    where
        P: Into<mint::Point2<u32>>,
    {
        let mirror_scale = voxel.mirror.into_scale();
        let dest = dest.into();
        let dest = mint::Point2::from([
            dest.x as f32 * self.tile_width(),
            dest.y as f32 * self.tile_height(),
        ]);

        let scale =
            mint::Vector2::from([mirror_scale.x * self.scaling, mirror_scale.y * self.scaling]);
        let offset = mint::Point2::from([0.0, 0.0]);

        self.fg_batch.add(
            DrawParam::new()
                .src(self.font.get_src_rect(voxel.char_offset))
                .dest(dest)
                .rotation(voxel.rotation.into_rotation())
                .scale(scale)
                .offset(offset)
                .color(voxel.foreground.into()),
        );

        if let Some(background) = voxel.background {
            self.bg_batch.add(
                DrawParam::new()
                    .src(Rect::new(0.0, 0.0, 1.0, 1.0))
                    .dest(dest)
                    .rotation(voxel.rotation.into_rotation())
                    .scale(mint::Vector2::from([
                        scale.x * self.font.char_width() as f32,
                        scale.y * self.font.char_height() as f32,
                    ]))
                    .offset(offset)
                    .color(background.into()),
            );
        }
    }

    pub fn clear(&mut self) {
        self.fg_batch.clear();
        self.bg_batch.clear();
    }
}

impl Drawable for KataFontBatch {
    fn draw(&self, ctx: &mut Context, param: DrawParam) -> GameResult {
        self.bg_batch.draw(ctx, param)?;
        self.fg_batch.draw(ctx, param)?;

        Ok(())
    }

    fn dimensions(&self, ctx: &mut Context) -> Option<Rect> {
        match (self.fg_batch.dimensions(ctx), self.bg_batch.dimensions(ctx)) {
            (None, None) => None,
            (Some(fg_dim), None) => Some(fg_dim),
            (None, Some(bg_dim)) => Some(bg_dim),
            (Some(fg_dim), Some(bg_dim)) => Some(fg_dim.combine_with(bg_dim)),
        }
    }

    fn set_blend_mode(&mut self, mode: Option<BlendMode>) {
        self.bg_batch.set_blend_mode(mode);
        self.fg_batch.set_blend_mode(mode);
    }

    fn blend_mode(&self) -> Option<BlendMode> {
        let fg_mode = self.fg_batch.blend_mode();
        let bg_mode = self.bg_batch.blend_mode();

        assert_eq!(fg_mode, bg_mode);

        fg_mode
    }
}
