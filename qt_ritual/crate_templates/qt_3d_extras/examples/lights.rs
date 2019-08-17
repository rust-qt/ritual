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

fn setup_scene(root: MutPtr<QEntity>) -> Box<Fn() -> ()> {
    unsafe {
        let mut scene = QEntity::new_1a(root);

        let mut sphere1 = QEntity::new_1a(&mut scene);
        let mut sphere1_mesh = QSphereMesh::new_0a();
        sphere1_mesh.set_radius(1.0);
        sphere1_mesh.set_rings(60);
        sphere1_mesh.set_slices(30);
        sphere1.add_component(sphere1_mesh.into_ptr());
        let mut sphere1_material = QMetalRoughMaterial::new_0a();
        sphere1_material.set_base_color_q_color(&QColor::from_rgb_3a(255, 255, 255));
        sphere1_material.set_metalness_float(0.5);
        sphere1_material.set_roughness_float(0.2);
        sphere1.add_component(sphere1_material.into_ptr());
        let mut sphere1_transform = QTransform::new_0a();
        sphere1_transform.set_translation(&QVector3D::from_3_float(-2.0, 0.0, 0.0));
        sphere1.add_component(sphere1_transform.into_ptr());
        sphere1.into_raw_ptr();

        let mut sphere2 = QEntity::new_1a(&mut scene);
        let mut sphere2_mesh = QSphereMesh::new_0a();
        sphere2_mesh.set_radius(1.0);
        sphere2_mesh.set_rings(60);
        sphere2_mesh.set_slices(30);
        sphere2.add_component(sphere2_mesh.into_ptr());
        let mut sphere2_material = QMetalRoughMaterial::new_0a();
        sphere2_material.set_base_color_q_color(&QColor::from_rgb_3a(255, 255, 255));
        sphere2_material.set_metalness_float(0.5);
        sphere2_material.set_roughness_float(0.2);
        sphere2.add_component(sphere2_material.into_ptr());
        let mut sphere2_transform = QTransform::new_0a();
        sphere2_transform.set_translation(&QVector3D::from_3_float(2.0, 0.0, 0.0));
        sphere2.add_component(sphere2_transform.into_ptr());
        sphere2.into_raw_ptr();

        let mut plane = QEntity::new_1a(&mut scene);
        plane.add_component(QPlaneMesh::new_0a().into_ptr());
        let mut plane_material = QMetalRoughMaterial::new_0a();
        plane_material.set_base_color_q_color(&QColor::from_rgb_3a(255, 255, 255));
        plane_material.set_metalness_float(0.5);
        plane_material.set_roughness_float(0.5);
        plane.add_component(plane_material.into_ptr());
        let mut plane_transform = QTransform::new_0a();
        plane_transform.set_scale(100.0);
        plane_transform.set_translation(&QVector3D::from_3_float(0.0, -2.0, 0.0));
        plane.add_component(plane_transform.into_ptr());
        plane.into_raw_ptr();

        let mut directional_light = QDirectionalLight::new_0a();
        directional_light.set_enabled(true);
        directional_light.set_color(&QColor::from_rgb_3a(255, 0, 0));
        directional_light.set_intensity(1.0);
        directional_light.set_world_direction(&QVector3D::from_3_float(1.0, -1.0, 0.0));
        let directional_light = directional_light.into_ptr();
        scene.add_component(directional_light);

        let mut point_light_entity = QEntity::new_1a(&mut scene);
        let mut point_light = QPointLight::new_0a();
        point_light.set_enabled(false);
        point_light.set_color(&QColor::from_rgb_3a(0, 255, 0));
        point_light.set_intensity(1.0);
        point_light.set_linear_attenuation(0.01);
        point_light.set_quadratic_attenuation(0.05);
        let point_light = point_light.into_ptr();
        point_light_entity.add_component(point_light);
        let mut point_light_transform = QTransform::new_0a();
        point_light_transform.set_translation(&QVector3D::from_3_float(0.0, 3.0, 1.0));
        point_light_entity.add_component(point_light_transform.into_ptr());
        point_light_entity.into_raw_ptr();

        let mut spot_light_entity = QEntity::new_1a(&mut scene);
        let mut spot_light = QSpotLight::new_0a();
        spot_light.set_enabled(false);
        spot_light.set_color(&QColor::from_rgb_3a(0, 0, 255));
        spot_light.set_intensity(1.0);
        spot_light.set_local_direction(&QVector3D::from_3_float(-1.0, -1.0, 0.0));
        spot_light.set_cut_off_angle(45.0);
        spot_light.set_linear_attenuation(0.05);
        spot_light.set_quadratic_attenuation(0.005);
        let spot_light = spot_light.into_ptr();
        spot_light_entity.add_component(spot_light);
        let mut spot_light_transform = QTransform::new_0a();
        spot_light_transform.set_translation(&QVector3D::from_3_float(6.0, 6.0, 0.0));
        spot_light_entity.add_component(spot_light_transform.into_ptr());
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
        QSurfaceFormat::set_default_format(&format);

        QGuiApplication::init(|_| {
            let mut window = Qt3DWindow::new_0a();

            let mut root = QEntity::new_0a();
            let activate_next_light = setup_scene(root.as_mut_ptr());
            let next_light_slot = Slot::new(activate_next_light.as_ref());

            let keyboard_device = QKeyboardDevice::new_1a(&mut root);
            let mut handler = QEntity::new_1a(&mut root);
            let mut keyboard_handler = QKeyboardHandler::new_0a();
            keyboard_handler.set_source_device(keyboard_device.into_ptr());
            keyboard_handler.set_focus(true);
            keyboard_handler.tab_pressed().connect(&next_light_slot);
            keyboard_handler.space_pressed().connect(&next_light_slot);
            handler.add_component(keyboard_handler.into_ptr());
            handler.into_ptr();

            let mut camera = window.camera();
            camera.set_position(&QVector3D::from_3_float(0.0, 0.0, 30.0));
            camera.set_view_center(&QVector3D::from_3_float(0.0, 0.0, 0.0));

            let mut controller = QOrbitCameraController::new_1a(&mut root);
            controller.set_camera(camera);
            controller.set_linear_speed(50.0);
            controller.set_look_speed(180.0);
            controller.into_ptr();

            window.set_root_entity(root.into_ptr());
            window.show();
            QGuiApplication::exec()
        })
    }
}
