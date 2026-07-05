const INPUT_OWNER_WEB: u8 = 1;
const INPUT_OWNER_HOST: u8 = 2;
const INPUT_REGION_SHAPE_RECT: u8 = 1;

pub fn input_ownership_map_from_legacy_pass_map(
    pass_map: ipc::OverlayPassMapOutput,
) -> ipc::InputOwnershipMapOutput {
    let viewport_width = pass_map.viewport_width;
    let viewport_height = pass_map.viewport_height;

    ipc::InputOwnershipMapOutput {
        version: pass_map.version,
        viewport_width,
        viewport_height,
        device_scale_factor: pass_map.device_scale_factor,
        enabled: pass_map.enabled,
        default_owner: INPUT_OWNER_WEB,
        regions: pass_map
            .regions
            .into_iter()
            .filter_map(|region| {
                legacy_pass_region_to_input_region(viewport_width, viewport_height, region)
            })
            .collect(),
    }
}

fn legacy_pass_region_to_input_region(
    viewport_width: i32,
    viewport_height: i32,
    region: ipc::OverlayPassRegionOutput,
) -> Option<ipc::InputOwnershipRegionOutput> {
    if region.disabled || region.shape != INPUT_REGION_SHAPE_RECT {
        return None;
    }

    if !is_valid_rect(region.x, region.y, region.width, region.height) {
        return None;
    }

    if region.x < 0.0
        || region.y < 0.0
        || region.x + region.width > viewport_width as f32
        || region.y + region.height > viewport_height as f32
    {
        return None;
    }

    Some(ipc::InputOwnershipRegionOutput {
        id: region.id,
        owner: INPUT_OWNER_HOST,
        shape: INPUT_REGION_SHAPE_RECT,
        disabled: false,
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
        radius: 0.0,
    })
}

fn is_valid_rect(x: f32, y: f32, width: f32, height: f32) -> bool {
    x.is_finite()
        && y.is_finite()
        && width.is_finite()
        && height.is_finite()
        && width > 0.0
        && height > 0.0
}

#[cfg(test)]
mod tests {
    use super::input_ownership_map_from_legacy_pass_map;

    #[test]
    fn maps_legacy_pass_regions_to_host_owned_input_regions() {
        let output = input_ownership_map_from_legacy_pass_map(ipc::OverlayPassMapOutput {
            version: 42,
            viewport_width: 1280,
            viewport_height: 720,
            device_scale_factor: 1.25,
            enabled: true,
            regions: vec![ipc::OverlayPassRegionOutput {
                id: 7,
                shape: 1,
                disabled: false,
                x: 10.5,
                y: 20.25,
                width: 300.0,
                height: 120.5,
            }],
        });

        assert_eq!(42, output.version);
        assert_eq!(1280, output.viewport_width);
        assert_eq!(720, output.viewport_height);
        assert_eq!(1.25, output.device_scale_factor);
        assert!(output.enabled);
        assert_eq!(1, output.default_owner);
        assert_eq!(1, output.regions.len());
        assert_eq!(7, output.regions[0].id);
        assert_eq!(2, output.regions[0].owner);
        assert_eq!(1, output.regions[0].shape);
        assert!(!output.regions[0].disabled);
        assert_eq!(10.5, output.regions[0].x);
        assert_eq!(20.25, output.regions[0].y);
        assert_eq!(300.0, output.regions[0].width);
        assert_eq!(120.5, output.regions[0].height);
        assert_eq!(0.0, output.regions[0].radius);
    }

    #[test]
    fn filters_disabled_invalid_and_out_of_viewport_regions() {
        let output = input_ownership_map_from_legacy_pass_map(ipc::OverlayPassMapOutput {
            version: 1,
            viewport_width: 100,
            viewport_height: 100,
            device_scale_factor: 1.0,
            enabled: false,
            regions: vec![
                ipc::OverlayPassRegionOutput {
                    id: 1,
                    shape: 1,
                    disabled: true,
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                },
                ipc::OverlayPassRegionOutput {
                    id: 2,
                    shape: 2,
                    disabled: false,
                    x: 0.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                },
                ipc::OverlayPassRegionOutput {
                    id: 3,
                    shape: 1,
                    disabled: false,
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 10.0,
                },
                ipc::OverlayPassRegionOutput {
                    id: 4,
                    shape: 1,
                    disabled: false,
                    x: 95.0,
                    y: 0.0,
                    width: 10.0,
                    height: 10.0,
                },
                ipc::OverlayPassRegionOutput {
                    id: 5,
                    shape: 1,
                    disabled: false,
                    x: 10.0,
                    y: 10.0,
                    width: 20.0,
                    height: 20.0,
                },
            ],
        });

        assert!(!output.enabled);
        assert_eq!(1, output.regions.len());
        assert_eq!(5, output.regions[0].id);
    }
}
