use std::thread::sleep;
use std::time::Duration;
use snowmix_conn::SnowmixConn;
use channel::Channel;
use monitor::Monitor;
use feeds::{Feed, VideoConfig};

pub struct Manager {
    snowmix: SnowmixConn,
    channels: Vec<Channel>,
    monitors: Vec<Monitor>,
    framerate: f32,
    video_config: VideoConfig,
}

impl Manager {
    pub fn new(snowmix_addr: &str, config: &VideoConfig) -> Manager {
        let snowmix = SnowmixConn::new(snowmix_addr);
        let mut channels: Vec<Channel> = Vec::new();
        let mut monitors: Vec<Monitor> = Vec::new();

        for i in 0..8 {
            channels.push(Channel {
                id: i,
                snowmix_id: i + 1,
                label: format!("{}", i + 1),
                is_preview: false,
                is_program: false,
                is_dsk: false
            });
        }

         Manager{snowmix: snowmix,
                 channels: channels,
                 monitors: monitors,
                 framerate: 30.0,
                 video_config: config.clone()}
    }

    pub fn get_channels_copy(&self) -> Vec<Channel> {
        self.channels.clone()
    }

    pub fn start(&mut self) {
        println!("{}", self.snowmix.info());
        self.set_program(0);
        self.set_preview(1);
    }

    fn set_program(&mut self, channel_id: usize) {
        self.set_program_without_update(channel_id);
        self.update_main_bus();
    }

    fn set_program_without_update(&mut self, channel_id: usize) {
        if channel_id >= self.channels.len() {
            return;
        }

        match self.get_program() {
            Some(x) => {x.is_program = false},
            None => {}
        }

        self.channels[channel_id].is_program = true;
    }

    pub fn set_preview(&mut self, channel_id: usize) {
        if channel_id >= self.channels.len() {
            return;
        }

        match self.get_preview() {
            Some(x) => {x.is_preview = false},
            None => {}
        }

        self.channels[channel_id].is_preview = true;
        self.update_main_bus();
    }

    fn update_main_bus(&mut self) {
        let program_id = match self.get_program() {
            Some(x) => x.snowmix_id,
            None => return,
        };

        let preview_id = match self.get_preview() {
            Some(x) => x.snowmix_id,
            None => return,
        };

        let dsk_feeds_list = self.build_dsks_feeds_list();

        self.snowmix.send(&format!("vfeed alpha {} 0", preview_id));
        self.snowmix.send(&format!("vfeed alpha {} 1", program_id));
        self.snowmix.send(&format!("tcl eval SetFeedToOverlay {} {} {}",
                                   program_id,
                                   preview_id,
                                   dsk_feeds_list));
    }

    pub fn take(&mut self) {
        let current_program_id = match self.get_program() {
            Some(x) => x.id,
            None => return
        };

        let current_preview_id = match self.get_preview() {
            Some(x) => x.id,
            None => return
        };

        self.set_program_without_update(current_preview_id as usize);
        self.set_preview(current_program_id as usize);
    }

    pub fn transition(&mut self, duration: f32) {
        let frames = (duration * self.framerate).ceil();
        let delta = 1. / frames;
        let snowmix_id = match self.get_preview() {
            Some(channel) => channel.snowmix_id,
            None => return
        };

        let msg = &format!("vfeed move alpha {} {} {}", snowmix_id, delta, frames);
        self.snowmix.send(msg);

       sleep(Duration::from_millis((duration * 1000.) as u64));
       self.take();
    }

    fn build_dsks_feeds_list(&self) -> String {
        let dsks: Vec<String> = self.channels.iter().filter_map(|ref channel| {
            match channel.is_dsk {
                true => Some(channel.snowmix_id.to_string()),
                false => None
            }
        }).collect();

        dsks.join(" ")
    }

    fn get_program(&mut self) -> Option<&mut Channel> {
        for channel in self.channels.iter_mut() {
            if channel.is_program {
                return Some(&mut *channel);
            }
        }
        None
    }

    fn get_preview(&mut self) -> Option<&mut Channel> {
        for channel in self.channels.iter_mut() {
            if channel.is_preview {
                return Some(&mut *channel);
            }
        }
        None
    }

    pub fn quit(&mut self) {
        for monitor in &mut self.monitors {
            monitor.stop();
        }

        self.snowmix.close();
    }

    pub fn create_monitor(&mut self, socket_path: String) {
        let mut monitor = Monitor::new(&socket_path, &self.video_config);
        monitor.play();
        self.monitors.push(monitor);
    }
}
