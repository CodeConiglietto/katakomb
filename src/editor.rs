use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use failure::Fallible;
use flo_binding::{Binding, Bound, MutableBound};
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
        voxel::{Model, Voxel2, Voxel3, VoxelFace},
    },
    ui::*,
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
                Image::solid(ctx, 1, graphics::Color::WHITE)?,
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

impl EventHandler<ggez::GameError> for Editor {
    fn mouse_wheel_event(&mut self, ctx: &mut Context, _x: f32, y: f32) {
        self.mouse_wheel_scroll += y;

        while self.mouse_wheel_scroll >= 1.0 {
            self.mouse_wheel_scroll -= 1.0;
            let event = Event::Mouse {
                pos: self.ui_context.mouse_pos(ctx),
                e: MouseEvent::WheelUp,
            };
            let layout_rect = self.layout_rect(ctx);
            let _ = self
                .mode
                .layout()
                .handle_event(&mut self.ui_context, event, layout_rect);
        }

        while self.mouse_wheel_scroll <= -1.0 {
            self.mouse_wheel_scroll += 1.0;
            let event = Event::Mouse {
                pos: self.ui_context.mouse_pos(ctx),
                e: MouseEvent::WheelDown,
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
            Event::Mouse {
                pos,
                e: MouseEvent::ButtonDown { button },
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
            Event::Mouse {
                pos,
                e: MouseEvent::ButtonUp { button },
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
                    Event::Mouse {
                        pos,
                        e: MouseEvent::ButtonDrag {
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
        graphics::clear(ctx, graphics::Color::BLACK);

        self.ui_context.batch.clear();

        let layout_rect = self.layout_rect(ctx);
        let _ = self
            .mode
            .layout()
            .handle_event(&mut self.ui_context, Event::Draw, layout_rect);

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
}

impl VoxelMode {
    fn new(voxel: Option<Voxel3>, font: &KataFont) -> Self {
        // Bindings
        let voxel = Binding::new(voxel.unwrap_or_else(Default::default));
        let active_face = Binding::new(VoxelFace::X);

        let charset_width = font.charset_width();

        // Layout
        let font_display = List::from_vec(
            (0..font.charset_height())
                .map(|y| {
                    ListElement::new(Box::new(
                        KataText::from_voxels(
                            (0..font.charset_width())
                                .map(|x| Voxel2::new(y * font.charset_width() + x))
                                .collect(),
                        )
                        .with_events({
                            let voxel = voxel.clone();
                            let active_face = active_face.clone();
                            move |_self, _ctx, e, bounds| {
                                match e.cull(bounds) {
                                    Some(Event::Mouse {
                                        pos,
                                        e:
                                            MouseEvent::ButtonDown {
                                                button: MouseButton::Left,
                                            },
                                    }) => {
                                        let x = pos.x - bounds.x;
                                        let char_offset = u16::from(y * charset_width) + x as u16;

                                        let mut new_voxel = voxel.get();
                                        new_voxel[active_face.get()].char_offset = char_offset;
                                        voxel.set(new_voxel);

                                        dbg!(char_offset);

                                        return Err(Stop);
                                    }

                                    _ => {}
                                }

                                Ok(Continue)
                            }
                        }),
                    ))
                })
                .collect(),
        );

        let face_display = |char_offset: u8, face: VoxelFace| {
            let voxel = voxel.clone();
            let active_face = active_face.clone();

            Box::new(FlexLayout::vertical(vec![
                FlexElement::fixed(Box::new(Placeholder::new(
                    Voxel2::new(char_offset.into()),
                    |c| dbg!(dbg!(c).constrain(Size::new(1, 1))),
                ))),
                FlexElement::fixed(Box::new(
                    VoxelDisplay::new(flo_binding::computed(move || voxel.get()[face].clone()))
                        .with_events(move |_self, _ctx, e, bounds| {
                            match e.cull(bounds) {
                                Some(Event::Mouse {
                                    e:
                                        MouseEvent::ButtonDown {
                                            button: MouseButton::Left,
                                        },
                                    ..
                                }) => active_face.set(face),

                                _ => {}
                            }

                            Ok(Continue)
                        }),
                )),
            ]))
        };

        let voxel_info = FlexLayout::vertical(vec![
            FlexElement::flex(Box::new(Filling::blank()), 1),
            FlexElement::fixed(Box::new(FlexLayout::horizontal(vec![
                FlexElement::flex(Box::new(Filling::blank()), 1),
                FlexElement::fixed(face_display(b'X', VoxelFace::X)),
                FlexElement::fixed(face_display(b'Y', VoxelFace::Y)),
                FlexElement::fixed(face_display(b'Z', VoxelFace::Z)),
                FlexElement::flex(Box::new(Filling::blank()), 1),
            ]))),
            FlexElement::flex(Box::new(Filling::blank()), 1),
        ]);

        let middle_pane = FlexLayout::vertical(vec![
            FlexElement::flex(Box::new(Centered::new(voxel_info)), 1),
            FlexElement::flex(placeholder(b'c', color::GREEN, |c| c.max), 1),
        ]);

        let voxel_list = List::from_vec(
            (1..=30)
                .map(|i| ListElement::new(Box::new(KataText::from_str(&format!("Voxel {}", i)))))
                .collect(),
        );

        Self {
            layout: FlexLayout::horizontal(vec![
                FlexElement::fixed(Box::new(font_display)),
                FlexElement::fixed(divider()),
                FlexElement::flex(Box::new(middle_pane), 1),
                FlexElement::fixed(divider()),
                FlexElement::flex(Box::new(voxel_list), 1),
            ]),
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
