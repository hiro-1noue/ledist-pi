use ledist_pi::{E233DisplaySelection, E233Layout, FieldSelection, plan_e233};

fn selection() -> E233DisplaySelection {
    E233DisplaySelection {
        service: FieldSelection::None,
        route: FieldSelection::None,
        service_change: FieldSelection::None,
        through_route: FieldSelection::None,
        destination: FieldSelection::None,
        scroll_text: String::new(),
        brightness: 40,
    }
}
#[test]
fn rejects_conflicting_destination_and_service_change() {
    let mut s = selection();
    s.destination = FieldSelection::Asset("d".into());
    s.service_change = FieldSelection::Asset("c".into());
    assert!(plan_e233(&s).is_err())
}
#[test]
fn makes_service_destination_scroll_page() {
    let mut s = selection();
    s.service = FieldSelection::Asset("s".into());
    s.destination = FieldSelection::Asset("d".into());
    s.scroll_text = "next".into();
    let plan = plan_e233(&s).unwrap();
    assert!(matches!(
        plan.pages.last().unwrap().layout,
        E233Layout::ServiceAndRightSplit(_, _, _)
    ));
}

#[test]
fn route_with_service_uses_a_split_page_before_the_scroll_page() {
    let selection = E233DisplaySelection {
        service: FieldSelection::Asset("local".into()),
        route: FieldSelection::Asset("saikyo".into()),
        service_change: FieldSelection::None,
        through_route: FieldSelection::None,
        destination: FieldSelection::None,
        scroll_text: "この電車は相鉄線へ直通します".into(),
        brightness: 40,
    };

    let plan = plan_e233(&selection).unwrap();
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
    assert!(matches!(
        plan.pages[1].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
}

#[test]
fn static_pages_follow_destination_route_through_change_order() {
    let mut s = selection();
    s.service = FieldSelection::Blank;
    s.destination = FieldSelection::Asset("d".into());
    s.route = FieldSelection::Asset("r".into());
    s.through_route = FieldSelection::Asset("t".into());
    let plan = plan_e233(&s).unwrap();
    assert_eq!(plan.pages.len(), 3);
    assert!(matches!(
        plan.pages[0].layout,
        E233Layout::ServiceAndRight(..)
    ));
    assert!(matches!(
        plan.pages[1].layout,
        E233Layout::ServiceAndRightSplit(..)
    ));
    assert!(matches!(
        plan.pages[2].layout,
        E233Layout::ServiceAndRight(..)
    ));
}

#[test]
fn rejects_invalid_scroll_and_service_change_combinations() {
    let mut s = selection();
    s.through_route = FieldSelection::Asset("t".into());
    s.scroll_text = "notice".into();
    assert!(plan_e233(&s).is_err());
    let mut s = selection();
    s.service_change = FieldSelection::Asset("c".into());
    assert!(plan_e233(&s).is_err());
}
