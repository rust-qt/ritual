use qt_3d_core::cpp_utils::*;
use qt_3d_core::*;
use qt_3d_extras::*;
use qt_3d_input::*;
use qt_3d_render::*;
use qt_core::*;
use qt_gui::{
    q_surface_format::OpenGLContextProfile, QColor, QGuiApplication, QSurfaceFormat, QVector3D,
};
use std::cell::*;

fn setup_scene(root: &mut QEntity) -> Box<Fn() -> ()> {
    unsafe {
        let mut scene = MutPtr(root.static_upcast_mut().into());

        let mut sphere1 = MutPtr(scene.static_upcast_mut().into());
        let mut sphere1_mesh = QSphereMesh::new_0a();
        sphere1_mesh.set_radius(1.0);
        sphere1_mesh.set_rings(60);
        sphere1_mesh.set_slices(30);
        sphere1.add_component(
            sphere1_mesh
                .into_ptr()
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );
        let mut sphere1_material = QMetalRoughMaterial::new_0a();
        sphere1_material.set_base_color(
            QColor::from_rgb_3a(255, 255, 255)
                .to_qt_core_q_variant()
                .as_ref(),
        );
        sphere1_material.set_metalness(QVariant::new12(0.5).as_ref());
        sphere1_material.set_roughness(QVariant::new12(0.2).as_ref());
        sphere1.add_component(
            sphere1_material
                .into_ptr()
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );
        let mut sphere1_transform = QTransform::new_0a();
        sphere1_transform.set_translation(QVector3D::new2(-2.0, 0.0, 0.0).as_ref());
        sphere1.add_component(sphere1_transform.into_ptr().static_upcast_mut().into());
        sphere1.into_raw_ptr();

        let mut sphere2 = MutPtr(scene.static_upcast_mut().into());
        let mut sphere2_mesh = QSphereMesh::new_0a();
        sphere2_mesh.set_radius(1.0);
        sphere2_mesh.set_rings(60);
        sphere2_mesh.set_slices(30);
        sphere2.add_component(
            sphere2_mesh
                .into_ptr()
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );
        let mut sphere2_material = QMetalRoughMaterial::new_0a();
        sphere2_material.set_base_color(
            QColor::from_rgb_3a(255, 255, 255)
                .to_qt_core_q_variant()
                .as_ref(),
        );
        sphere2_material.set_metalness(QVariant::new12(0.5).as_ref());
        sphere2_material.set_roughness(QVariant::new12(0.2).as_ref());
        sphere2.add_component(
            sphere2_material
                .into_ptr()
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );
        let mut sphere2_transform = QTransform::new_0a();
        sphere2_transform.set_translation(QVector3D::new2(2.0, 0.0, 0.0).as_ref());
        sphere2.add_component(sphere2_transform.into_ptr().static_upcast_mut().into());
        sphere2.into_raw_ptr();

        let mut plane = MutPtr(scene.static_upcast_mut().into());
        plane.add_component(
            QPlaneMesh::new_0a()
                .into_ptr()
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );
        let mut plane_material = QMetalRoughMaterial::new_0a();
        plane_material.set_base_color(
            QColor::from_rgb_3a(255, 255, 255)
                .to_qt_core_q_variant()
                .as_ref(),
        );
        plane_material.set_metalness(QVariant::new12(0.5).as_ref());
        plane_material.set_roughness(QVariant::new12(0.5).as_ref());
        plane.add_component(
            plane_material
                .into_ptr()
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );
        let mut plane_transform = QTransform::new_0a();
        plane_transform.set_scale(100.0);
        plane_transform.set_translation(QVector3D::new2(0.0, -2.0, 0.0).as_ref());
        plane.add_component(plane_transform.into_ptr().static_upcast_mut().into());
        plane.into_raw_ptr();

        let mut directional_light = QDirectionalLight::new_0a();
        directional_light.set_enabled(true);
        directional_light.set_color(QColor::from_rgb_3a(255, 0, 0).as_ref());
        directional_light.set_intensity(1.0);
        directional_light.set_world_direction(QVector3D::new2(1.0, -1.0, 0.0).as_ref());
        let mut directional_light = directional_light.into_ptr();
        scene.add_component(
            directional_light
                .static_upcast_mut()
                .static_upcast_mut()
                .into(),
        );

        let mut point_light_entity = MutPtr(scene.static_upcast_mut().into());
        let mut point_light = QPointLight::new_0a();
        point_light.set_enabled(false);
        point_light.set_color(QColor::from_rgb_3a(0, 255, 0).as_ref());
        point_light.set_intensity(1.0);
        point_light.set_linear_attenuation(0.01);
        point_light.set_quadratic_attenuation(0.05);
        let mut point_light = point_light.into_ptr();
        point_light_entity
            .add_component(point_light.static_upcast_mut().static_upcast_mut().into());
        let mut point_light_transform = QTransform::new_0a();
        point_light_transform.set_translation(QVector3D::new2(0.0, 3.0, 1.0).as_ref());
        point_light_entity
            .add_component(point_light_transform.into_ptr().static_upcast_mut().into());
        point_light_entity.into_raw_ptr();

        let mut spot_light_entity = MutPtr(scene.static_upcast_mut().into());
        let mut spot_light = QSpotLight::new_0a();
        spot_light.set_enabled(false);
        spot_light.set_color(QColor::from_rgb_3a(0, 0, 255).as_ref());
        spot_light.set_intensity(1.0);
        spot_light.set_local_direction(QVector3D::new2(-1.0, -1.0, 0.0).as_ref());
        spot_light.set_cut_off_angle(45.0);
        spot_light.set_linear_attenuation(0.05);
        spot_light.set_quadratic_attenuation(0.005);
        let mut spot_light = spot_light.into_ptr();
        spot_light_entity.add_component(spot_light.static_upcast_mut().static_upcast_mut().into());
        let mut spot_light_transform = QTransform::new_0a();
        spot_light_transform.set_translation(QVector3D::new2(6.0, 6.0, 0.0).as_ref());
        spot_light_entity.add_component(spot_light_transform.into_ptr().static_upcast_mut().into());
        spot_light_entity.into_raw_ptr();

        scene.into_raw_ptr();

        let active_light_cell = RefCell::new(0);
        Box::new(move || {
            let active_light = (*active_light_cell.borrow() + 1) % 3;
            directional_light.clone().set_enabled(active_light == 0);
            point_light.clone().set_enabled(active_light == 1);
            spot_light.clone().set_enabled(active_light == 2);
            *active_light_cell.borrow_mut() = active_light;
        })
    }
}

fn main() {
    unsafe {
        let mut format = QSurfaceFormat::new_0a();
        format.set_version(3, 3);
        format.set_profile(OpenGLContextProfile::CoreProfile);
        format.set_depth_buffer_size(24);
        format.set_samples(4);
        format.set_stencil_buffer_size(8);
        QSurfaceFormat::set_default_format(format.as_ref());

        QGuiApplication::init(|_| {
            let mut window = Qt3DWindow::new_0a();

            let mut root = QEntity::new_0a();
            let activate_next_light = setup_scene(&mut root);
            let next_light_slot = Slot::new(activate_next_light.as_ref());

            let mut keyboard_device = QKeyboardDevice::new_1a(root.static_upcast_mut().into());
            let mut handler = MutPtr(root.static_upcast_mut().into());
            let mut keyboard_handler = QKeyboardHandler::new_0a();
            keyboard_handler.set_source_device(keyboard_device.as_mut_ptr());
            keyboard_handler.set_focus(true);
            keyboard_handler.tab_pressed().connect(&next_light_slot);
            keyboard_handler.space_pressed().connect(&next_light_slot);
            handler.add_component(keyboard_handler.static_upcast_mut().into());
            handler.into_raw_ptr();

            let mut camera = window.camera();
            camera.set_position(QVector3D::new2(0.0, 0.0, 30.0).as_ref());
            camera.set_view_center(QVector3D::new2(0.0, 0.0, 0.0).as_ref());

            let mut controller = QOrbitCameraController::new_1a(root.static_upcast_mut().into());
            controller.set_camera(camera);
            controller.set_linear_speed(50.0);
            controller.set_look_speed(180.0);
            controller.into_raw_ptr();

            window.set_root_entity(root.into_ptr());

            let mut window = window.into_ptr();
            window.show();
            QGuiApplication::exec()
        })
    }
}
