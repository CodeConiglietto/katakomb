use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use failure::Fallible;
use ggez::{
    event::EventHandler,
    graphics::{self, DrawParam, Image},
    input::mouse::MouseButton,
    Context, GameResult,
};
use internship::IStr;
use log::debug;
use na::Point3;
use ndarray::Array3;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    geometry::rect::IRect,
    rendering::{
        color::{self, Color},
        font::{KataFont, KataFontBatch},
        voxel::{Model, Voxel2, Voxel3},
    },
    ui::{
        self, BoxConstraints, Element, ElementExt, FlexElement, FlexLayout, KataText,
        LayoutDirection, List, ListElement, Placeholder, Size, StackedElement, StackedLayout,
        UiContext,
    },
};

pub struct Editor {
    voxels: BTreeMap<IStr, Voxel3>,
    models: BTreeMap<IStr, Model>,

    recent: Recent,

    ui_context: UiContext,
    mode: EditorMode,

    held_buttons: HashMap<MouseButton, HeldButton>,
    mouse_wheel_scroll: f32,
}

impl Editor {
    pub fn new(ctx: &mut Context) -> Fallible<Self> {
        let voxels: BTreeMap<IStr, Voxel3> = try_load("voxels.json")?;
        let models: BTreeMap<IStr, Model> = try_load("models.json")?;
        let recent: Recent = try_load(".recent.json")?;
        let font = KataFont::load(ctx)?;

        Ok(Self {
            mode: EditorMode::restore(&recent, &voxels, &models, &font),
            ui_context: UiContext::new(KataFontBatch::new(
                font,
                Image::solid(ctx, 1, graphics::WHITE)?,
                4.0,
            )),

            voxels,
            models,
            recent,

            mouse_wheel_scroll: 0.0,
            held_buttons: HashMap::new(),
        })
    }

    fn layout_size(&self, ctx: &Context) -> Size {
        let screen_size = graphics::drawable_size(ctx);
        Size::new(
            (screen_size.0 / self.ui_context.batch.tile_width()) as u32,
            (screen_size.1 / self.ui_context.batch.tile_height()) as u32,
        )
    }

    fn layout_rect(&self, ctx: &Context) -> IRect {
        let layout_size = self.layout_size(ctx);
        IRect::new(0, 0, layout_size.width, layout_size.height)
    }
}

impl EventHandler for Editor {
    fn mouse_wheel_event(&mut self, ctx: &mut Context, _x: f32, y: f32) {
        self.mouse_wheel_scroll += y;

        while self.mouse_wheel_scroll >= 1.0 {
            self.mouse_wheel_scroll -= 1.0;
            let event = ui::Event::Mouse {
                pos: self.ui_context.mouse_pos(ctx),
                e: ui::MouseEvent::WheelUp,
            };
            let layout_rect = self.layout_rect(ctx);
            let _ = self
                .mode
                .layout()
                .handle_event(&mut self.ui_context, event, layout_rect);
        }

        while self.mouse_wheel_scroll <= -1.0 {
            self.mouse_wheel_scroll += 1.0;
            let event = ui::Event::Mouse {
                pos: self.ui_context.mouse_pos(ctx),
                e: ui::MouseEvent::WheelDown,
            };
            let layout_rect = self.layout_rect(ctx);
            let _ = self
                .mode
                .layout()
                .handle_event(&mut self.ui_context, event, layout_rect);
        }
    }

    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        let pos = dbg!(self.ui_context.mouse_pos(ctx));
        self.held_buttons.insert(
            button,
            HeldButton {
                pos,
                start_pos: pos,
            },
        );

        let layout_rect = self.layout_rect(ctx);
        let _ = self.mode.layout().handle_event(
            &mut self.ui_context,
            ui::Event::Mouse {
                pos,
                e: ui::MouseEvent::ButtonDown { button },
            },
            layout_rect,
        );
    }

    fn mouse_button_up_event(&mut self, ctx: &mut Context, button: MouseButton, _x: f32, _y: f32) {
        self.held_buttons.remove(&button);

        let pos = self.ui_context.mouse_pos(ctx);
        let layout_rect = self.layout_rect(ctx);
        let _ = self.mode.layout().handle_event(
            &mut self.ui_context,
            ui::Event::Mouse {
                pos,
                e: ui::MouseEvent::ButtonUp { button },
            },
            layout_rect,
        );
    }

    fn mouse_motion_event(&mut self, ctx: &mut Context, _x: f32, _y: f32, _dx: f32, _dy: f32) {
        let pos = self.ui_context.mouse_pos(ctx);
        let layout_rect = self.layout_rect(ctx);

        for (&button, held) in self.held_buttons.iter_mut() {
            if pos != held.pos {
                held.pos = pos;

                let _ = self.mode.layout().handle_event(
                    &mut self.ui_context,
                    ui::Event::Mouse {
                        pos,
                        e: ui::MouseEvent::ButtonDrag {
                            button,
                            start_pos: held.start_pos,
                        },
                    },
                    layout_rect,
                );
            }
        }
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        if self.ui_context.relayout {
            debug!("Relayout");
            self.ui_context.relayout = false;
            let layout_size = self.layout_size(ctx);
            self.mode
                .layout()
                .layout(BoxConstraints::exact(layout_size));
        }

        match &mut self.mode {
            EditorMode::Voxel(voxel_mode) => {}

            EditorMode::Model(model_mode) => {}
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::clear(ctx, graphics::BLACK);

        self.ui_context.batch.clear();

        let layout_rect = self.layout_rect(ctx);
        let _ = self
            .mode
            .layout()
            .handle_event(&mut self.ui_context, ui::Event::Draw, layout_rect);

        match &mut self.mode {
            EditorMode::Voxel(voxel_mode) => {}

            EditorMode::Model(model_mode) => {}
        }

        graphics::draw(ctx, &self.ui_context.batch, DrawParam::default())?;
        graphics::present(ctx)?;

        Ok(())
    }

    fn resize_event(&mut self, _ctx: &mut Context, _width: f32, _height: f32) {
        self.ui_context.relayout = true;
    }
}

#[derive(Clone, Copy, Debug)]
struct HeldButton {
    start_pos: mint::Point2<u32>,
    pos: mint::Point2<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct Recent {
    voxel: Option<IStr>,
    model: Option<IStr>,
    mode: EditorModeName,
}

fn try_load<T, P>(path: P) -> Fallible<T>
where
    T: DeserializeOwned + Default,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.is_file() {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    } else {
        Ok(T::default())
    }
}

enum EditorMode {
    Voxel(VoxelMode),
    Model(ModelMode),
}

impl EditorMode {
    fn layout(&mut self) -> &mut dyn Element {
        match self {
            EditorMode::Voxel(v) => &mut v.layout,
            EditorMode::Model(m) => &mut m.layout,
        }
    }

    fn name(&self) -> EditorModeName {
        match self {
            EditorMode::Voxel(_) => EditorModeName::Voxel,
            EditorMode::Model(_) => EditorModeName::Model,
        }
    }

    fn restore(
        recent: &Recent,
        voxels: &BTreeMap<IStr, Voxel3>,
        models: &BTreeMap<IStr, Model>,
        font: &KataFont,
    ) -> Self {
        match recent.mode {
            EditorModeName::Voxel => EditorMode::Voxel(VoxelMode::new(
                recent.voxel.as_ref().and_then(|v| voxels.get(v)).cloned(),
                font,
            )),
            EditorModeName::Model => EditorMode::Model(ModelMode::new(
                recent
                    .model
                    .as_ref()
                    .and_then(|m| models.get(m))
                    .cloned()
                    .map(EditableModel::from),
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
enum EditorModeName {
    Voxel,
    Model,
}

impl Default for EditorModeName {
    fn default() -> Self {
        EditorModeName::Voxel
    }
}

struct VoxelMode {
    layout: FlexLayout,
    current: Arc<Mutex<Option<Voxel3>>>,
}

impl VoxelMode {
    fn new(current_voxel: Option<Voxel3>, font: &KataFont) -> Self {
        let current = Arc::new(Mutex::new(current_voxel));

        let charset_width = font.charset_width();

        let font_display = List::from_vec(
            (0..font.charset_height())
                .map(|y| {
                    ListElement::new(Box::new(
                        KataText::from_voxels(
                            (0..font.charset_width())
                                .map(|x| Voxel2::new(y * font.charset_width() + x))
                                .collect(),
                        )
                        .with_events(move |_self, _ctx, e, bounds| {
                            match e.cull(bounds) {
                                Some(ui::Event::Mouse { pos, e }) => match e {
                                    ui::MouseEvent::ButtonDown { button } => match button {
                                        MouseButton::Left => {
                                            let x = pos.x - bounds.x;
                                            let char_offset = u32::from(y * charset_width) + x;

                                            dbg!(char_offset);

                                            return Err(ui::Stop);
                                        }
                                        _ => {}
                                    },
                                    _ => {}
                                },

                                _ => {}
                            }

                            Ok(ui::Continue)
                        }),
                    ))
                })
                .collect(),
        );

        Self {
            layout: FlexLayout::from_vec(
                LayoutDirection::Horizontal,
                vec![
                    FlexElement::fixed(Box::new(font_display)),
                    FlexElement::fixed(divider()),
                    FlexElement::flex(
                        Box::new(FlexLayout::from_vec(
                            LayoutDirection::Vertical,
                            vec![
                                FlexElement::flex(placeholder(b'b', color::RED, |c| c.max), 1),
                                FlexElement::flex(placeholder(b'c', color::GREEN, |c| c.max), 1),
                            ],
                        )),
                        1,
                    ),
                    FlexElement::fixed(divider()),
                    FlexElement::flex(
                        Box::new(List::from_vec(
                            (1..=30)
                                .map(|i| {
                                    ListElement::new(Box::new(KataText::from_str(&format!(
                                        "Voxel {}",
                                        i
                                    ))))
                                })
                                .collect(),
                        )),
                        1,
                    ),
                ],
            ),

            current,
        }
    }
}

fn placeholder<N, F>(char_offset: N, color: Color, size_fn: F) -> Box<dyn Element>
where
    N: Into<u16>,
    F: Fn(BoxConstraints) -> Size + 'static,
{
    Box::new(Placeholder::new(
        Voxel2::new(char_offset.into()).background(Some(color)),
        size_fn,
    ))
}

fn divider() -> Box<dyn Element> {
    placeholder(0x266u16, color::BLACK, |c| Size::new(1, c.max.height))
}

struct ModelMode {
    layout: StackedLayout,
    current: Arc<Mutex<Option<Model>>>,
}

impl ModelMode {
    fn new(current_model: Option<EditableModel>) -> Self {
        todo!()
    }
}

impl From<EditableModel> for Model {
    fn from(mut eo: EditableModel) -> Self {
        if eo.voxels.is_empty() {
            return Self {
                voxels: Array3::from_shape_simple_fn((0, 0, 0), || unreachable!()),
            };
        }

        let mut keys = eo.voxels.keys();
        let first = keys.by_ref().next().unwrap();

        let mut min_x = first.coords.x;
        let mut min_y = first.coords.y;
        let mut min_z = first.coords.z;

        let mut max_x = first.coords.x;
        let mut max_y = first.coords.y;
        let mut max_z = first.coords.z;

        for pos in keys {
            min_x = min_x.min(pos.coords.x);
            min_y = min_y.min(pos.coords.y);
            min_z = min_z.min(pos.coords.z);

            max_x = max_x.max(pos.coords.x);
            max_y = max_y.max(pos.coords.y);
            max_z = max_z.max(pos.coords.z);
        }

        let w = (max_x - min_x) as usize;
        let h = (max_y - min_y) as usize;
        let d = (max_z - min_z) as usize;

        let voxels = Array3::from_shape_fn((w, h, d), |(x, y, z)| {
            eo.voxels.remove(&Point3::new(
                (x as i16) - min_x,
                (y as i16) - min_y,
                (z as i16) - min_z,
            ))
        });

        assert!(eo.voxels.is_empty());

        Self { voxels }
    }
}

#[derive(Clone, Debug)]
struct EditableModel {
    voxels: HashMap<Point3<i16>, IStr>,
}

impl From<Model> for EditableModel {
    fn from(mut o: Model) -> Self {
        Self {
            voxels: o
                .voxels
                .indexed_iter_mut()
                .filter_map(|((x, y, z), v)| {
                    v.take().map(|v| {
                        (
                            Point3::new(
                                x.try_into().unwrap(),
                                y.try_into().unwrap(),
                                z.try_into().unwrap(),
                            ),
                            v,
                        )
                    })
                })
                .collect(),
        }
    }
}
