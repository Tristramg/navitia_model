// Copyright 2017-2018 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see
// <http://www.gnu.org/licenses/>.

use super::{Agency, DirectionType, Stop, Trip};
use collection::CollectionWithId;
use common_format::Availability;
use csv;
use failure::ResultExt;
use objects;
use std::path;
use Result;

pub fn write_agencies(
    path: &path::Path,
    networks: &CollectionWithId<objects::Network>,
) -> Result<()> {
    info!("Writing agency.txt");
    let path = path.join("agency.txt");
    let mut wtr = csv::Writer::from_path(&path).with_context(ctx_from_path!(path))?;
    for n in networks.values() {
        wtr.serialize(Agency::from(n))
            .with_context(ctx_from_path!(path))?;
    }

    wtr.flush().with_context(ctx_from_path!(path))?;

    Ok(())
}

pub fn write_stops(
    path: &path::Path,
    stop_points: &CollectionWithId<objects::StopPoint>,
    stop_areas: &CollectionWithId<objects::StopArea>,
) -> Result<()> {
    info!("Writing stops.txt");
    let path = path.join("stops.txt");
    let mut wtr = csv::Writer::from_path(&path).with_context(ctx_from_path!(path))?;
    for sp in stop_points.values() {
        wtr.serialize(Stop::from(sp))
            .with_context(ctx_from_path!(path))?;
    }
    for sa in stop_areas.values() {
        wtr.serialize(Stop::from(sa))
            .with_context(ctx_from_path!(path))?;
    }

    wtr.flush().with_context(ctx_from_path!(path))?;

    Ok(())
}

fn get_gtfs_trip_shortname_and_headsign_from_ntfs_vj(
    vj: &objects::VehicleJourney,
    sps: &CollectionWithId<objects::StopPoint>,
) -> (Option<String>, Option<String>) {
    fn get_last_stop_name(
        vj: &objects::VehicleJourney,
        sps: &CollectionWithId<objects::StopPoint>,
    ) -> Option<String> {
        vj.stop_times
            .iter()
            .max_by_key(|st| st.sequence)
            .map(|st| &sps[st.stop_point_idx].name)
            .cloned()
    }

    match vj.physical_mode_id.as_str() {
        "LocalTrain" | "LongDistanceTrain" | "Metro" | "RapidTransit" | "Train" => {
            (vj.headsign.clone(), get_last_stop_name(vj, sps))
        }
        _ => (
            None,
            vj.headsign.clone().or_else(|| get_last_stop_name(vj, sps)),
        ),
    }
}

fn get_gtfs_direction_id_from_ntfs_vj(
    vj: &objects::VehicleJourney,
    routes: &CollectionWithId<objects::Route>,
) -> DirectionType {
    let route = routes.get(&vj.route_id).unwrap();
    match route.direction_type.as_ref().map(|s| s.as_str()) {
        Some("foward") | Some("clockwise") | Some("inbound") => DirectionType::Backward,
        _ => DirectionType::Forward,
    }
}

fn make_gtfs_trip_from_ntfs_vj(
    vj: &objects::VehicleJourney,
    sps: &CollectionWithId<objects::StopPoint>,
    routes: &CollectionWithId<objects::Route>,
    tps: &CollectionWithId<objects::TripProperty>,
) -> Trip {
    let (short_name, headsign) = get_gtfs_trip_shortname_and_headsign_from_ntfs_vj(vj, sps);
    let mut wheelchair_and_bike = (Availability::default(), Availability::default());
    if let Some(tp_id) = &vj.trip_property_id {
        if let Some(tp) = tps.get(&tp_id) {
            wheelchair_and_bike = (tp.wheelchair_accessible, tp.bike_accepted);
        };
    }

    Trip {
        route_id: vj.route_id.clone(),
        service_id: vj.service_id.clone(),
        id: vj.id.clone(),
        headsign,
        short_name,
        direction: get_gtfs_direction_id_from_ntfs_vj(vj, routes),
        block_id: vj.block_id.clone(),
        shape_id: vj.geometry_id.clone(),
        wheelchair_accessible: wheelchair_and_bike.0,
        bikes_allowed: wheelchair_and_bike.1,
    }
}

pub fn write_trips(
    path: &path::Path,
    vjs: &CollectionWithId<objects::VehicleJourney>,
    sps: &CollectionWithId<objects::StopPoint>,
    routes: &CollectionWithId<objects::Route>,
    tps: &CollectionWithId<objects::TripProperty>,
) -> Result<()> {
    info!("Writing trips.txt");
    let path = path.join("trips.txt");
    let mut wtr = csv::Writer::from_path(&path).with_context(ctx_from_path!(path))?;
    for vj in vjs.values() {
        wtr.serialize(make_gtfs_trip_from_ntfs_vj(vj, sps, routes, tps))
            .with_context(ctx_from_path!(path))?;
    }

    wtr.flush().with_context(ctx_from_path!(path))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtfs::StopLocationType;
    use std::collections::BTreeSet;

    #[test]
    fn write_agency() {
        let agency = Agency::from(&objects::Network {
            id: "OIF:101".to_string(),
            name: "SAVAC".to_string(),
            url: Some("http://www.vianavigo.com,Europe/Paris".to_string()),
            timezone: Some("Europe/Madrid".to_string()),
            lang: Some("fr".to_string()),
            phone: Some("0123456789".to_string()),
            address: Some("somewhere".to_string()),
            sort_order: Some(1),
            codes: Default::default(),
        });

        let expected_agency = Agency {
            id: Some("OIF:101".to_string()),
            name: "SAVAC".to_string(),
            url: "http://www.vianavigo.com,Europe/Paris".to_string(),
            timezone: "Europe/Madrid".to_string(),
            lang: Some("fr".to_string()),
            phone: Some("0123456789".to_string()),
            email: None,
        };

        assert_eq!(expected_agency, agency);
    }

    #[test]
    fn write_agency_with_default_values() {
        let agency = Agency::from(&objects::Network {
            id: "OIF:101".to_string(),
            name: "SAVAC".to_string(),
            url: None,
            timezone: None,
            lang: None,
            phone: None,
            address: None,
            sort_order: None,
            codes: Default::default(),
        });

        let expected_agency = Agency {
            id: Some("OIF:101".to_string()),
            name: "SAVAC".to_string(),
            url: "http://www.navitia.io/".to_string(),
            timezone: "Europe/Paris".to_string(),
            lang: None,
            phone: None,
            email: None,
        };

        assert_eq!(expected_agency, agency);
    }

    #[test]
    fn ntfs_stop_point_to_gtfs_stop() {
        let stop = Stop::from(&objects::StopPoint {
            id: "sp_1".to_string(),
            name: "sp_name_1".to_string(),
            codes: BTreeSet::default(),
            object_properties: BTreeSet::default(),
            comment_links: BTreeSet::default(),
            visible: true,
            coord: objects::Coord {
                lon: 2.073034,
                lat: 48.799115,
            },
            stop_area_id: "OIF:SA:8739322".to_string(),
            timezone: Some("Europe/Paris".to_string()),
            geometry_id: None,
            equipment_id: None,
            fare_zone_id: Some("1".to_string()),
        });

        let expected = Stop {
            id: "sp_1".to_string(),
            name: "sp_name_1".to_string(),
            lat: 48.799115,
            lon: 2.073034,
            fare_zone_id: Some("1".to_string()),
            location_type: StopLocationType::StopPoint,
            parent_station: Some("OIF:SA:8739322".to_string()),
            code: None,
            desc: "".to_string(),
            wheelchair_boarding: None,
            url: None,
            timezone: Some("Europe/Paris".to_string()),
        };

        assert_eq!(expected, stop);

        // with no timezone and fare_zone_id
        let stop = Stop::from(&objects::StopPoint {
            id: "sp_1".to_string(),
            name: "sp_name_1".to_string(),
            codes: BTreeSet::default(),
            object_properties: BTreeSet::default(),
            comment_links: BTreeSet::default(),
            visible: true,
            coord: objects::Coord {
                lon: 2.073034,
                lat: 48.799115,
            },
            stop_area_id: "OIF:SA:8739322".to_string(),
            timezone: None,
            geometry_id: None,
            equipment_id: None,
            fare_zone_id: None,
        });

        let expected = Stop {
            id: "sp_1".to_string(),
            name: "sp_name_1".to_string(),
            lat: 48.799115,
            lon: 2.073034,
            fare_zone_id: None,
            location_type: StopLocationType::StopPoint,
            parent_station: Some("OIF:SA:8739322".to_string()),
            code: None,
            desc: "".to_string(),
            wheelchair_boarding: None,
            url: None,
            timezone: None,
        };

        assert_eq!(expected, stop);
    }

    #[test]
    fn ntfs_stop_area_to_gtfs_stop() {
        let stop = Stop::from(&objects::StopArea {
            id: "sa_1".to_string(),
            name: "sa_name_1".to_string(),
            codes: BTreeSet::default(),
            object_properties: BTreeSet::default(),
            comment_links: BTreeSet::default(),
            visible: true,
            coord: objects::Coord {
                lon: 2.073034,
                lat: 48.799115,
            },
            timezone: Some("Europe/Paris".to_string()),
            geometry_id: None,
            equipment_id: None,
        });

        let expected = Stop {
            id: "sa_1".to_string(),
            name: "sa_name_1".to_string(),
            lat: 48.799115,
            lon: 2.073034,
            fare_zone_id: None,
            location_type: StopLocationType::StopArea,
            parent_station: None,
            code: None,
            desc: "".to_string(),
            wheelchair_boarding: None,
            url: None,
            timezone: Some("Europe/Paris".to_string()),
        };

        assert_eq!(expected, stop);
    }

    #[test]
    fn write_trip() {
        let sps = CollectionWithId::new(vec![
            objects::StopPoint {
                id: "OIF:SP:36:2085".to_string(),
                name: "Gare de Saint-Cyr l'École".to_string(),
                codes: BTreeSet::default(),
                object_properties: BTreeSet::default(),
                comment_links: BTreeSet::default(),
                visible: true,
                coord: objects::Coord {
                    lon: 2.073034,
                    lat: 48.799115,
                },
                stop_area_id: "OIF:SA:8739322".to_string(),
                timezone: Some("Europe/Paris".to_string()),
                geometry_id: None,
                equipment_id: None,
                fare_zone_id: Some("1".to_string()),
            },
            objects::StopPoint {
                id: "OIF:SP:36:2127".to_string(),
                name: "Division Leclerc".to_string(),
                codes: BTreeSet::default(),
                object_properties: BTreeSet::default(),
                comment_links: BTreeSet::default(),
                visible: true,
                coord: objects::Coord {
                    lon: 2.073407,
                    lat: 48.800598,
                },
                stop_area_id: "OIF:SA:2:1468".to_string(),
                timezone: Some("Europe/Paris".to_string()),
                geometry_id: None,
                equipment_id: None,
                fare_zone_id: None,
            },
        ]).unwrap();
        let routes = CollectionWithId::new(vec![objects::Route {
            id: "OIF:078078001:1".to_string(),
            name: "Hôtels - Hôtels".to_string(),
            direction_type: Some("foward".to_string()),
            codes: BTreeSet::default(),
            object_properties: BTreeSet::default(),
            comment_links: BTreeSet::default(),
            line_id: "OIF:002002002:BDEOIF829".to_string(),
            geometry_id: Some("Geometry:Line:Relation:6883353".to_string()),
            destination_id: Some("OIF,OIF:SA:4:126".to_string()),
        }]).unwrap();

        let tps = CollectionWithId::new(vec![objects::TripProperty {
            id: "1".to_string(),
            wheelchair_accessible: Availability::Available,
            bike_accepted: Availability::NotAvailable,
            air_conditioned: Availability::InformationNotAvailable,
            visual_announcement: Availability::Available,
            audible_announcement: Availability::Available,
            appropriate_escort: Availability::Available,
            appropriate_signage: Availability::Available,
            school_vehicle_type: objects::TransportType::Regular,
        }]).unwrap();
        let vj = objects::VehicleJourney {
            id: "OIF:87604986-1_11595-1".to_string(),
            codes: BTreeSet::default(),
            object_properties: BTreeSet::default(),
            comment_links: BTreeSet::default(),
            route_id: "OIF:078078001:1".to_string(),
            physical_mode_id: "Bus".to_string(),
            dataset_id: "OIF:0".to_string(),
            service_id: "2".to_string(),
            headsign: Some("2005".to_string()),
            block_id: Some("PLOI".to_string()),
            company_id: "OIF:743".to_string(),
            trip_property_id: Some("1".to_string()),
            geometry_id: Some("Geometry:Line:Relation:6883353".to_string()),
            stop_times: vec![
                objects::StopTime {
                    stop_point_idx: sps.get_idx("OIF:SP:36:2085").unwrap(),
                    sequence: 0,
                    arrival_time: objects::Time::new(14, 40, 0),
                    departure_time: objects::Time::new(14, 40, 0),
                    boarding_duration: 0,
                    alighting_duration: 0,
                    pickup_type: 0,
                    drop_off_type: 1,
                    datetime_estimated: false,
                    local_zone_id: None,
                },
                objects::StopTime {
                    stop_point_idx: sps.get_idx("OIF:SP:36:2127").unwrap(),
                    sequence: 1,
                    arrival_time: objects::Time::new(14, 42, 0),
                    departure_time: objects::Time::new(14, 42, 0),
                    boarding_duration: 0,
                    alighting_duration: 0,
                    pickup_type: 0,
                    drop_off_type: 0,
                    datetime_estimated: false,
                    local_zone_id: None,
                },
            ],
        };

        let expected = Trip {
            route_id: "OIF:078078001:1".to_string(),
            service_id: vj.service_id.clone(),
            id: "OIF:87604986-1_11595-1".to_string(),
            headsign: Some("2005".to_string()),
            short_name: None,
            direction: DirectionType::Backward,
            block_id: Some("PLOI".to_string()),
            shape_id: vj.geometry_id.clone(),
            wheelchair_accessible: Availability::Available,
            bikes_allowed: Availability::NotAvailable,
        };

        assert_eq!(
            expected,
            make_gtfs_trip_from_ntfs_vj(&vj, &sps, &routes, &tps)
        );
    }
}
