use parking_lot::Mutex;
use shaders::Sphere;
use std::{cmp::max, sync::Arc, thread::JoinHandle};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::DeviceExtensions,
    instance::debug::{DebugCallback, MessageSeverity, MessageType},
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    sync::{self, GpuFuture},
};

use spirv_std::glam::Vec3;

pub struct Vk {
    pub tx: std::sync::mpsc::Sender<Command>,
    pub rx: std::sync::mpsc::Receiver<Response>,
    pub handle: JoinHandle<()>,
}

pub enum Command {
    Break,
    Scale(f32),
    RefreshPrimary(Arc<Mutex<Vec<Vec3>>>),
    LoadPrimary(Arc<Mutex<Vec<Vec3>>>),
    Spheres(shaders::RayConstants, Vec<Sphere>),
}

pub enum Response {
    Scale,
    RefreshPrimary,
    SingleSphere,
    LoadPrimary,
    Break,
}

#[repr(C)]
struct ScalePush {
    global_mult: f32,
    w: u32,
    h: u32,
    _pad: u32,
}

impl Vk {
    pub fn cmd(&mut self, c: Command) {
        self.tx.send(c).unwrap();
        self.rx.recv().unwrap();
    }

    pub fn new() -> Vk {
        let (cmd_tx, rx) = std::sync::mpsc::channel();
        let (tx, resp_rx) = std::sync::mpsc::channel();

        let app = vulkano::instance::InstanceCreateInfo {
            application_name: Some("shitting furiously".into()),

            enabled_layers: vec![],
            ..Default::default()
        };

        let inst = vulkano::instance::Instance::new(app).expect("no vulkan!");
        let _callback =
            DebugCallback::new(&inst, MessageSeverity::all(), MessageType::all(), |msg| {
                println!("Debug callback: {:?}", msg.description);
            })
            .ok();
        let phys = vulkano::device::physical::PhysicalDevice::enumerate(&inst);

        let phy_dev = phys
            .filter(|d| {
                d.properties().device_type
                    == vulkano::device::physical::PhysicalDeviceType::DiscreteGpu
            })
            .next()
            .expect("No discrete GPUs found!");

        let queue_fams = phy_dev.queue_families();

        let compute = queue_fams
            .filter(|q| q.supports_compute())
            .next()
            .expect("No compute devices!");

        let (dev, mut queues) = vulkano::device::Device::new(
            phy_dev,
            vulkano::device::DeviceCreateInfo {
                queue_create_infos: vec![vulkano::device::QueueCreateInfo::family(compute)],
                enabled_extensions: {
                    let mut v = DeviceExtensions::none();
                    //v.khr_portability_subset = true;
                    v.ext_scalar_block_layout = true;
                    v.khr_vulkan_memory_model = true;
                    v
                },
                ..Default::default()
            },
        )
        .expect("device construction failed");
        println! {"{:?}", dev.enabled_extensions()};

        let queue = queues.next().expect("no queues?");
        let _sm = unsafe {
            vulkano::shader::ShaderModule::from_bytes(
                dev.clone(),
                include_bytes!("../../shaders/scale.spirv"),
            )
        }
        .expect("failed to read shader, recompile?");

        let ray_shaders = unsafe {
            vulkano::shader::ShaderModule::from_bytes(
                dev.clone(),
                include_bytes!("../../shaders/ray.spirv"),
            )
        }
        .expect("failed to read shader, recompile?");

        let scale_entry = _sm.entry_point("main").expect("missing main");

        let ray_entry = ray_shaders
            .entry_point("single_ray")
            .expect("missing single_ray");

        let data_buffer = {
            let data_iter = (0..crate::W * crate::H).map(|_| [0.0f32; 3]);
            CpuAccessibleBuffer::from_iter(
                dev.clone(),
                BufferUsage {
                    storage_buffer: true,
                    ..BufferUsage::none()
                },
                false,
                data_iter,
            )
            .unwrap()
        };

        let spheres_buffer = {
            let data_iter = (0..crate::W * crate::H).map(|_| [0.0f32; 4]);
            CpuAccessibleBuffer::from_iter(
                dev.clone(),
                BufferUsage {
                    storage_buffer: true,
                    ..BufferUsage::none()
                },
                false,
                data_iter,
            )
            .unwrap()
        };

        let scale_pipeline =
            { ComputePipeline::new(dev.clone(), scale_entry, &(), None, |_| {}).unwrap() };

        let scale_layout = scale_pipeline.layout().set_layouts().get(0).unwrap();
        let scale_set = PersistentDescriptorSet::new(
            scale_layout.clone(),
            [WriteDescriptorSet::buffer(0, data_buffer.clone())],
        )
        .unwrap();

        let ray_pipeline =
            { ComputePipeline::new(dev.clone(), ray_entry, &(), None, |_| {}).unwrap() };

        let ray_layout = ray_pipeline.layout().set_layouts().get(0).unwrap();
        let ray_set = PersistentDescriptorSet::new(
            ray_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, data_buffer.clone()),
                WriteDescriptorSet::buffer(1, spheres_buffer.clone()),
            ],
        )
        .unwrap();

        let handle = std::thread::spawn(move || loop {
            match rx.recv().expect("GPU process receiver error") {
                Command::Scale(param) => {
                    let mut builder = AutoCommandBufferBuilder::primary(
                        dev.clone(),
                        queue.family(),
                        CommandBufferUsage::OneTimeSubmit,
                    )
                    .unwrap();
                    builder
                        .bind_pipeline_compute(scale_pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Compute,
                            scale_pipeline.layout().clone(),
                            0,
                            scale_set.clone(),
                        )
                        .push_constants(
                            scale_pipeline.layout().clone(),
                            0,
                            ScalePush {
                                global_mult: param,
                                w: crate::W as u32,
                                h: crate::H as u32,
                                _pad: 0,
                            },
                        )
                        .dispatch([crate::W as u32 / 32, crate::H as u32, 1])
                        .unwrap();

                    // Finish building the command buffer by calling `build`.
                    let scale_cmdbuf = builder.build().unwrap();

                    let future = sync::now(dev.clone())
                        .then_execute(queue.clone(), scale_cmdbuf)
                        .unwrap()
                        .then_signal_fence_and_flush()
                        .unwrap();

                    future.wait(None).unwrap();
                    tx.send(Response::Scale).unwrap();
                }
                Command::RefreshPrimary(img_buf) => {
                    let img_buf = img_buf.lock();
                    let mut data = data_buffer.write().unwrap();
                    for y in 0..crate::H {
                        for x in 0..crate::W {
                            let pix = img_buf[y * crate::W + x];
                            data[y * crate::W + x][0] = pix.x;
                            data[y * crate::W + x][1] = pix.y;
                            data[y * crate::W + x][2] = pix.z;
                        }
                    }
                    tx.send(Response::RefreshPrimary).unwrap();
                }
                Command::LoadPrimary(img_buf) => {
                    let mut img_buf = img_buf.lock();
                    let data = data_buffer.read().unwrap();
                    for y in 0..crate::H {
                        for x in 0..crate::W {
                            let pix = data[y * crate::W + x];
                            img_buf[y * crate::W + x] = Vec3::new(pix[0], pix[1], pix[2]);
                        }
                    }
                    tx.send(Response::LoadPrimary).unwrap();
                }
                Command::Spheres(scene, spheres) => {
                    let mut data = spheres_buffer.write().unwrap();
                    for (ix, sphere) in spheres.into_iter().enumerate() {
                        for i in 0..3 {
                            data[ix][i] = sphere.center[i];
                        }
                        data[ix][3] = sphere.radius;
                    }
                    drop(data);
                    let mut builder = AutoCommandBufferBuilder::primary(
                        dev.clone(),
                        queue.family(),
                        CommandBufferUsage::OneTimeSubmit,
                    )
                    .unwrap();
                    //println!("{:?}", ray_pipeline.layout());
                    builder
                        .bind_pipeline_compute(ray_pipeline.clone())
                        .bind_descriptor_sets(
                            PipelineBindPoint::Compute,
                            ray_pipeline.layout().clone(),
                            0,
                            ray_set.clone(),
                        )
                        .push_constants(ray_pipeline.layout().clone(), 0, scene)
                        .dispatch([crate::W as u32 / 32, crate::H as u32, scene.nsample])
                        .unwrap();

                    let ray_cmdbuf = builder.build().unwrap();

                    let future = sync::now(dev.clone())
                        .then_execute(queue.clone(), ray_cmdbuf)
                        .unwrap()
                        .then_signal_fence_and_flush()
                        .unwrap();

                    future.wait(None).unwrap();
                    tx.send(Response::SingleSphere).unwrap();
                }
                Command::Break => {
                    tx.send(Response::Break).unwrap();
                    break;
                }
            }
        });

        Vk {
            tx: cmd_tx,
            rx: resp_rx,
            handle,
        }
    }
}
