pub trait InputDevice {
    fn write(&mut self, val: u8);

    fn read(&mut self) -> u8;

    // Todo: Not general!
    fn set_state(&mut self, state: ControllerState);
}

#[derive(Default, Clone, Copy)]
pub struct ControllerState {
    pub up: bool,
    pub down: bool,
    pub right: bool,
    pub left: bool,
    pub start: bool,
    pub select: bool,
    pub a: bool,
    pub b: bool,
}

#[derive(Default)]
pub struct Controller {
    button_state: ControllerState,
    shift_register: u8,
    should_poll: bool,
}

impl Controller {
    fn poll(&mut self) {
        let mut val = 0;
        val |= if self.button_state.a { 0x01 } else { 0 };
        val |= if self.button_state.b { 0x02 } else { 0 };
        val |= if self.button_state.select { 0x04 } else { 0 };
        val |= if self.button_state.start { 0x08 } else { 0 };
        val |= if self.button_state.up { 0x10 } else { 0 };
        val |= if self.button_state.down { 0x20 } else { 0 };
        val |= if self.button_state.left { 0x40 } else { 0 };
        val |= if self.button_state.right { 0x80 } else { 0 };
        self.shift_register = val;
    }
}

impl InputDevice for Controller {
    fn write(&mut self, val: u8) {
        self.should_poll = val & 0x01 != 0;
        if self.should_poll {
            self.poll()
        }
    }

    fn read(&mut self) -> u8 {
        if self.should_poll {
            self.poll();
        }
        let ret = self.shift_register & 0x01;
        self.shift_register >>= 1;
        self.shift_register |= 0x80; // Official controllers read 1 after emptying
        ret
    }

    fn set_state(&mut self, state: ControllerState) {
        self.button_state = state
    }
}

#[derive(Default)]
pub struct InputBus {
    controller1: Option<Box<dyn InputDevice>>,
    controller2: Option<Box<dyn InputDevice>>,
}

impl InputBus {
    pub fn new() -> Self {
        Self {
            controller1: Some(Box::<Controller>::default()),
            controller2: None,
        }
    }

    pub fn write(&mut self, val: u8) {
        if let Some(c) = &mut self.controller1 {
            c.write(val & 0x01)
        }
        if let Some(c) = &mut self.controller2 {
            c.write(val & 0x01)
        }
    }

    pub fn read_4016(&mut self) -> u8 {
        match &mut self.controller1 {
            Some(c) => c.read(),
            None => 0,
        }
    }

    pub fn read_4017(&mut self) -> u8 {
        match &mut self.controller2 {
            Some(c) => c.read(),
            None => 0,
        }
    }

    // TODO: not general!
    pub fn set_controller1_state(&mut self, state: ControllerState) {
        if let Some(c) = &mut self.controller1 {
            c.set_state(state)
        }
    }
}
