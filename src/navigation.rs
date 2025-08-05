use crate::screens;
use gtk4::{prelude::*, Box as GtkBox, Orientation, Stack, StackSidebar};

pub struct Navigation {
    pub container: GtkBox,
    pub stack: Stack,
}

impl Navigation {
    pub fn new() -> Self {
        // Create the stack for main content
        let stack = Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .transition_duration(300)
            .build();

        // Add detailed pages from dashboard.rs
        stack.add_titled(
            &screens::appearance::appearance_page(),
            Some("appearance"),
            "Appearance",
        );
        stack.add_titled(
            &screens::keyboard::keyboard_page(),
            Some("keyboard"),
            "Keyboard",
        );
        stack.add_titled(
            &screens::display::display_page(),
            Some("display"),
            "Display",
        );
        stack.add_titled(
            &screens::network::network_page(),
            Some("network"),
            "Network",
        );

        stack.add_titled(
            &screens::plugins::plugins_page(),
            Some("plugins"),
            "Plugins",
        );
        stack.add_titled(&screens::about::about_page(), Some("about"), "About");

        // Create the sidebar and link it to the stack
        let sidebar = StackSidebar::builder().stack(&stack).build();
        sidebar.set_vexpand(true);
        sidebar.set_hexpand(false);

        // Layout: horizontal box with sidebar and stack
        let container = GtkBox::new(Orientation::Horizontal, 0);
        container.append(&sidebar);
        container.append(&stack);

        Self { container, stack }
    }
    pub fn widget(&self) -> &GtkBox {
        &self.container
    }
    pub fn stack(&self) -> &Stack {
        &self.stack
    }
}
