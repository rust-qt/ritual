extern crate qt_3d_core;
extern crate qt_3d_extras;
extern crate qt_3d_input;
extern crate qt_3d_logic;
extern crate qt_3d_render;
extern crate qt_core;
extern crate qt_gui;

use std::cell::*;

use qt_core::connection::*;
use qt_core::slots::*;

use qt_gui::color::*;
use qt_gui::gui_application::GuiApplication;
use qt_gui::surface_format::*;
use qt_gui::vector_3d::*;

use qt_3d_core::cpp_utils::*;
use qt_3d_core::entity::*;
use qt_3d_core::node::*;
use qt_3d_core::transform::*;

use qt_3d_input::keyboard_device::*;
use qt_3d_input::keyboard_handler::*;

use qt_3d_render::directional_light::*;
use qt_3d_render::point_light::*;
use qt_3d_render::spot_light::*;

use qt_3d_extras::metal_rough_material::*;
use qt_3d_extras::orbit_camera_controller::*;
use qt_3d_extras::plane_mesh::*;
use qt_3d_extras::qt_3d_window::*;
use qt_3d_extras::sphere_mesh::*;

fn to_node_ptr(entity: &mut Entity) -> *mut Node {
    static_cast_mut::<Node, Entity>(entity)
}

fn setup_scene(root: &mut Entity) -> Box<Fn() -> ()> {
    let mut scene = unsafe { Entity::new_unsafe(to_node_ptr(root)) };

    let mut sphere1 = unsafe { Entity::new_unsafe(to_node_ptr(scene.as_mut())) };
    let mut sphere1_mesh = SphereMesh::new();
    sphere1_mesh.set_radius(1.0);
    sphere1_mesh.set_rings(60);
    sphere1_mesh.set_slices(30);
    unsafe { sphere1.add_component(static_cast_mut(sphere1_mesh.into_raw())) };
    let mut sphere1_material = MetalRoughMaterial::new();
    sphere1_material.set_base_color(&Color::from_rgb((255, 255, 255)));
    sphere1_material.set_metalness(0.5);
    sphere1_material.set_roughness(0.2);
    unsafe { sphere1.add_component(static_cast_mut(sphere1_material.into_raw())) };
    let mut sphere1_transform = Transform::new();
    sphere1_transform.set_translation(&Vector3D::new((-2.0, 0.0, 0.0)));
    unsafe { sphere1.add_component(static_cast_mut(sphere1_transform.into_raw())) };
    sphere1.into_raw();

    let mut sphere2 = unsafe { Entity::new_unsafe(to_node_ptr(scene.as_mut())) };
    let mut sphere2_mesh = SphereMesh::new();
    sphere2_mesh.set_radius(1.0);
    sphere2_mesh.set_rings(60);
    sphere2_mesh.set_slices(30);
    unsafe { sphere2.add_component(static_cast_mut(sphere2_mesh.into_raw())) };
    let mut sphere2_material = MetalRoughMaterial::new();
    sphere2_material.set_base_color(&Color::from_rgb((255, 255, 255)));
    sphere2_material.set_metalness(0.5);
    sphere2_material.set_roughness(0.2);
    unsafe { sphere2.add_component(static_cast_mut(sphere2_material.into_raw())) };
    let mut sphere2_transform = Transform::new();
    sphere2_transform.set_translation(&Vector3D::new((2.0, 0.0, 0.0)));
    unsafe { sphere2.add_component(static_cast_mut(sphere2_transform.into_raw())) };
    sphere2.into_raw();

    let mut plane = unsafe { Entity::new_unsafe(to_node_ptr(scene.as_mut())) };
    unsafe { plane.add_component(static_cast_mut(PlaneMesh::new().into_raw())) };
    let mut plane_material = MetalRoughMaterial::new();
    plane_material.set_base_color(&Color::from_rgb((255, 255, 255)));
    plane_material.set_metalness(0.5);
    plane_material.set_roughness(0.5);
    unsafe { plane.add_component(static_cast_mut(plane_material.into_raw())) };
    let mut plane_transform = Transform::new();
    plane_transform.set_scale(100.0);
    plane_transform.set_translation(&Vector3D::new((0.0, -2.0, 0.0)));
    unsafe { plane.add_component(static_cast_mut(plane_transform.into_raw())) };
    plane.into_raw();

    let mut directional_light = DirectionalLight::new();
    directional_light.set_enabled(true);
    directional_light.set_color(&Color::from_rgb((255, 0, 0)));
    directional_light.set_intensity(1.0);
    directional_light.set_world_direction(&Vector3D::new((1.0, -1.0, 0.0)));
    unsafe { scene.add_component(static_cast_mut(directional_light.as_mut_ptr())) };

    let mut point_light_entity = unsafe { Entity::new_unsafe(to_node_ptr(scene.as_mut())) };
    let mut point_light = PointLight::new();
    point_light.set_enabled(false);
    point_light.set_color(&Color::from_rgb((0, 255, 0)));
    point_light.set_intensity(1.0);
    point_light.set_linear_attenuation(0.01);
    point_light.set_quadratic_attenuation(0.05);
    unsafe { point_light_entity.add_component(static_cast_mut(point_light.as_mut_ptr())) };
    let mut point_light_transform = Transform::new();
    point_light_transform.set_translation(&Vector3D::new((0.0, 3.0, 1.0)));
    unsafe { point_light_entity.add_component(static_cast_mut(point_light_transform.into_raw())) };
    point_light_entity.into_raw();

    let mut spot_light_entity = unsafe { Entity::new_unsafe(to_node_ptr(scene.as_mut())) };
    let mut spot_light = SpotLight::new();
    spot_light.set_enabled(false);
    spot_light.set_color(&Color::from_rgb((0, 0, 255)));
    spot_light.set_intensity(1.0);
    spot_light.set_local_direction(&Vector3D::new((-1.0, -1.0, 0.0)));
    spot_light.set_cut_off_angle(45.0);
    spot_light.set_linear_attenuation(0.05);
    spot_light.set_quadratic_attenuation(0.005);
    unsafe { spot_light_entity.add_component(static_cast_mut(spot_light.as_mut_ptr())) };
    let mut spot_light_transform = Transform::new();
    spot_light_transform.set_translation(&Vector3D::new((6.0, 6.0, 0.0)));
    unsafe { spot_light_entity.add_component(static_cast_mut(spot_light_transform.into_raw())) };
    spot_light_entity.into_raw();

    scene.into_raw();

    let directional_light = directional_light.into_raw();
    let point_light = point_light.into_raw();
    let spot_light = spot_light.into_raw();

    let active_light_cell = RefCell::new(0);
    Box::new(move || {
        let active_light = (*active_light_cell.borrow() + 1) % 3;
        unsafe {
            (*directional_light).set_enabled(active_light == 0);
            (*point_light).set_enabled(active_light == 1);
            (*spot_light).set_enabled(active_light == 2);
        }
        *active_light_cell.borrow_mut() = active_light;
    })
}

fn main() {
    let mut format = SurfaceFormat::new(());
    format.set_version(3, 3);
    format.set_profile(OpenGLContextProfile::Core);
    format.set_depth_buffer_size(24);
    format.set_samples(4);
    format.set_stencil_buffer_size(8);
    SurfaceFormat::set_default_format(&format);

    GuiApplication::create_and_exit(|_| {
        let mut window = Qt3DWindow::new();

        let mut root = Entity::new();
        let activate_next_light = setup_scene(&mut root);
        let next_light_slot = SlotNoArgs::new(activate_next_light.as_ref());

        let keyboard_device = unsafe { KeyboardDevice::new_unsafe(to_node_ptr(root.as_mut())) };
        let mut handler = unsafe { Entity::new_unsafe(to_node_ptr(root.as_mut())) };
        let mut keyboard_handler = KeyboardHandler::new();
        unsafe { keyboard_handler.set_source_device(keyboard_device.into_raw()) };
        keyboard_handler.set_focus(true);
        keyboard_handler
            .signals()
            .tab_pressed()
            .connect(&next_light_slot);
        keyboard_handler
            .signals()
            .space_pressed()
            .connect(&next_light_slot);
        unsafe { handler.add_component(static_cast_mut(keyboard_handler.into_raw())) };
        handler.into_raw();

        let camera = window.camera();
        unsafe { (*camera).set_position(&Vector3D::new((0.0, 0.0, 30.0))) };
        unsafe { (*camera).set_view_center(&Vector3D::new((0.0, 0.0, 0.0))) };

        let mut controller = unsafe { OrbitCameraController::new_unsafe(to_node_ptr(&mut root)) };
        unsafe { controller.set_camera(camera) };
        controller.set_linear_speed(50.0);
        controller.set_look_speed(180.0);
        controller.into_raw();

        unsafe { window.set_root_entity(root.into_raw()) };

        window.show();
        GuiApplication::exec()
    })
}
