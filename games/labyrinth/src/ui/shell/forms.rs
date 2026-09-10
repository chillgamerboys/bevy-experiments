//! Game-owned admission forms and provider-neutral session browser.

use super::*;

pub(super) fn field(
    world: &mut World,
    parent: Entity,
    name: &str,
    field: Field,
    value: &str,
    max: usize,
) {
    label(
        world,
        parent,
        &format!("{name} Label"),
        name,
        UiTextRole::Supporting,
    );
    let bundle = text_field(world.resource::<UiFonts>(), name, value, max);
    world.spawn((
        bundle,
        UiSkin::Field,
        field,
        UiFocusId::new("labyrinth-form", format!("{field:?}")),
        ChildOf(parent),
    ));
}

pub(super) fn host_form(world: &mut World, parent: Entity, ui: &UiState) {
    let form = surface(world, parent, "Host Form");
    label(
        world,
        form,
        "Host Title",
        "ASSEMBLE YOUR COMPANY",
        UiTextRole::Title,
    );
    field(
        world,
        form,
        "Session name",
        Field::Name,
        &ui.session_name,
        64,
    );
    field(
        world,
        form,
        "Advertised address (empty: choose automatically)",
        Field::Address,
        &ui.address,
        253,
    );
    field(
        world,
        form,
        "Game UDP port (empty: 7777)",
        Field::Port,
        &ui.port,
        5,
    );
    field(
        world,
        form,
        "Temporary passphrase (visible while typing)",
        Field::Password,
        &ui.password.0,
        64,
    );
    label(world, form, "Password Warning", "Discovery requires 8-64 printable characters. Use a temporary session passphrase, never an account password. Direct invitations do not require it.", UiTextRole::Supporting);
    let discovery = row(world, form, "Discovery Settings");
    control(
        world,
        discovery,
        "LAN Discovery",
        if ui.lan {
            "LAN discovery: ON"
        } else {
            "LAN discovery: OFF"
        },
        Action::ToggleLan,
        false,
    );
    control(
        world,
        discovery,
        "Tailnet Discovery",
        if ui.tailnet {
            "Tailnet (Dev): ON"
        } else {
            "Tailnet (Dev): OFF"
        },
        Action::ToggleTailnet,
        false,
    );
    label(world, form, "Host Advice", "Three independent private invitation codes will be available in the lobby. Tailnet discovery requires external Tailscale installation and login.", UiTextRole::Supporting);
    let actions = row(world, form, "Host Actions");
    control(
        world,
        actions,
        "Create Session",
        "Light the lanterns | Host",
        Action::Host,
        false,
    );
    control(
        world,
        actions,
        "Host Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
}

pub(super) fn direct_form(world: &mut World, parent: Entity, ui: &UiState) {
    let form = surface(world, parent, "Direct Form");
    label(
        world,
        form,
        "Direct Title",
        "PRIVATE INVITATION",
        UiTextRole::Title,
    );
    label(world, form, "Code Advice", "Ask the host for your own BGN1 code. Each invitation admits one player; do not share another guest's code. The code is cleared after the attempt.", UiTextRole::Supporting);
    field(
        world,
        form,
        "Private BGN1 connection code",
        Field::Code,
        &ui.code.0,
        2048,
    );
    let actions = row(world, form, "Direct Actions");
    control(
        world,
        actions,
        "Submit Direct",
        "Join direct",
        Action::JoinCode,
        false,
    );
    control(
        world,
        actions,
        "Direct Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
}

pub(super) fn browser(world: &mut World, parent: Entity, view: &LabyrinthView, ui: &UiState) {
    let form = surface(world, parent, "Session Browser");
    label(
        world,
        form,
        "Browser Title",
        "COMPANIES NEARBY",
        UiTextRole::Title,
    );
    label(world, form, "Browser Advice", "Listings are availability hints, not admission. LAN is local multicast; Tailnet uses explicitly enabled development discovery.", UiTextRole::Supporting);
    let actions = row(world, form, "Browser Actions");
    control(
        world,
        actions,
        "Browser Refresh",
        "Refresh",
        Action::Browse,
        false,
    );
    control(
        world,
        actions,
        "Browser Tailnet",
        if ui.tailnet {
            "Tailnet browser: ON"
        } else {
            "Tailnet browser: OFF"
        },
        Action::ToggleTailnet,
        false,
    );
    control(
        world,
        actions,
        "Browser Direct",
        "Use private code",
        Action::Form(Form::Direct),
        false,
    );
    control(
        world,
        actions,
        "Browser Back",
        "Back",
        Action::Form(Form::Menu),
        false,
    );
    if view.listings.is_empty() {
        label(world, form, "Empty Browser", "No companies are visible yet. Keep this browser open, verify host discovery is enabled, or use a private code.", UiTextRole::Body);
    }
    for (index, listing) in view.listings.iter().enumerate() {
        label(
            world,
            form,
            &format!("Listing {index} Description"),
            listing.label.clone(),
            UiTextRole::Body,
        );
        control(
            world,
            form,
            format!("Join Listing {index}"),
            if listing.compatible {
                "Join locked company"
            } else {
                "Incompatible game or rules"
            },
            Action::Session(listing.id),
            !listing.compatible,
        );
    }
}

pub(super) fn password_form(world: &mut World, parent: Entity, ui: &UiState) {
    let form = surface(world, parent, "Discovery Password Form");
    label(
        world,
        form,
        "Discovery Password Title",
        "ENTER THE COMPANY PASSPHRASE",
        UiTextRole::Title,
    );
    field(
        world,
        form,
        "Temporary passphrase (visible while typing)",
        Field::Password,
        &ui.password.0,
        64,
    );
    label(world, form, "Discovery Password Advice", "Sent only after the selected host's encrypted certificate-pinned connection opens. Do not reuse an important password.", UiTextRole::Supporting);
    let actions = row(world, form, "Password Actions");
    control(
        world,
        actions,
        "Submit Discovery",
        "Join selected company",
        Action::JoinDiscovered,
        ui.selected_session.is_none(),
    );
    control(
        world,
        actions,
        "Password Back",
        "Back to companies",
        Action::Form(Form::Browser),
        false,
    );
}
