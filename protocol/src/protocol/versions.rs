use super::*;

mod v15w39c;
mod v18w50a;
mod v19w02a;
mod v1_10_2;
mod v1_11_2;
mod v1_12_2;
mod v1_13_2;
mod v1_14;
mod v1_14_1;
mod v1_14_2;
mod v1_14_3;
mod v1_14_4;
mod v1_15;
mod v1_16_1;
mod v1_16_4;
mod v1_17_1;
mod v1_18_1;
mod v1_18_2;
mod v1_20_1;
mod v1_7_10;
mod v1_8_9;
mod v1_9;
mod v1_9_2;

// https://wiki.vg/Protocol_History
// https://wiki.vg/Protocol_version_numbers#Versions_after_the_Netty_rewrite

pub fn protocol_name_to_protocol_version(s: String) -> i32 {
    match s.as_ref() {
        "" => SUPPORTED_PROTOCOLS[0],
        "1.20.1" => 763,
        "1.18.2" => 758,
        "1.18.1" => 757,
        "1.17.1" => 756,
        "1.16.5" => 754,
        "1.16.4" => 754,
        "1.16.3" => 753,
        "1.16.2" => 751,
        "1.16.1" => 736,
        "1.16" => 735,
        "1.15.2" => 578,
        "1.15.1" => 575,
        "1.14.4" => 498,
        "1.14.3" => 490,
        "1.14.2" => 485,
        "1.14.1" => 480,
        "1.14" => 477,
        "19w02a" => 452,
        "18w50a" => 451,
        "1.13.2" => 404,
        "1.12.2" => 340,
        "1.11.2" => 316,
        "1.11" => 315,
        "1.10.2" => 210,
        "1.9.2" => 109,
        "1.9" => 107,
        "15w39c" => 74,
        "1.8.9" => 47,
        "1.7.10" => 5,
        _ => {
            if let Ok(n) = s.parse::<i32>() {
                n
            } else {
                panic!("Unrecognized protocol name: {}", s)
            }
        }
    }
}

pub fn translate_internal_packet_id_for_version(
    version: i32,
    state: State,
    dir: Direction,
    id: i32,
    to_internal: bool,
) -> i32 {
    match version {
        763 => v1_20_1::translate_internal_packet_id(state, dir, id, to_internal),
        758 => v1_18_2::translate_internal_packet_id(state, dir, id, to_internal),
        757 => v1_18_1::translate_internal_packet_id(state, dir, id, to_internal),
        756 => v1_17_1::translate_internal_packet_id(state, dir, id, to_internal),
        754 | 753 | 751 => v1_16_4::translate_internal_packet_id(state, dir, id, to_internal),
        736 => v1_16_1::translate_internal_packet_id(state, dir, id, to_internal),
        735 => v1_16_1::translate_internal_packet_id(state, dir, id, to_internal),
        578 => v1_15::translate_internal_packet_id(state, dir, id, to_internal),
        575 => v1_15::translate_internal_packet_id(state, dir, id, to_internal),
        498 => v1_14_4::translate_internal_packet_id(state, dir, id, to_internal),
        490 => v1_14_3::translate_internal_packet_id(state, dir, id, to_internal),
        485 => v1_14_2::translate_internal_packet_id(state, dir, id, to_internal),
        480 => v1_14_1::translate_internal_packet_id(state, dir, id, to_internal),
        477 => v1_14::translate_internal_packet_id(state, dir, id, to_internal),
        452 => v19w02a::translate_internal_packet_id(state, dir, id, to_internal),
        451 => v18w50a::translate_internal_packet_id(state, dir, id, to_internal),
        404 => v1_13_2::translate_internal_packet_id(state, dir, id, to_internal),
        340 => v1_12_2::translate_internal_packet_id(state, dir, id, to_internal),
        316 => v1_11_2::translate_internal_packet_id(state, dir, id, to_internal),
        315 => v1_11_2::translate_internal_packet_id(state, dir, id, to_internal),
        210 => v1_10_2::translate_internal_packet_id(state, dir, id, to_internal),
        109 => v1_9_2::translate_internal_packet_id(state, dir, id, to_internal),
        107 => v1_9::translate_internal_packet_id(state, dir, id, to_internal),
        74 => v15w39c::translate_internal_packet_id(state, dir, id, to_internal),
        47 => v1_8_9::translate_internal_packet_id(state, dir, id, to_internal),
        5 => v1_7_10::translate_internal_packet_id(state, dir, id, to_internal),
        _ => panic!("unsupported protocol version: {}", version),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_name_accepts_valence_current_1_20_1() {
        assert_eq!(protocol_name_to_protocol_version("1.20.1".to_string()), 763);
    }

    #[test]
    fn protocol_763_reuses_1_18_2_handshake_translation() {
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Handshaking,
                Direction::Serverbound,
                0,
                true,
            ),
            translate_internal_packet_id_for_version(
                758,
                State::Handshaking,
                Direction::Serverbound,
                0,
                true,
            )
        );
    }

    #[test]
    fn protocol_763_uses_optional_uuid_login_start() {
        assert_eq!(
            translate_internal_packet_id_for_version(763, State::Login, Direction::Serverbound, 0x00, true),
            crate::protocol::packet::login::serverbound::internal_ids::LoginStart_WithOptionalUuid,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Login,
                Direction::Serverbound,
                crate::protocol::packet::login::serverbound::internal_ids::LoginStart_WithOptionalUuid,
                false,
            ),
            0x00,
        );
        assert_ne!(
            translate_internal_packet_id_for_version(763, State::Login, Direction::Serverbound, 0x00, true),
            translate_internal_packet_id_for_version(758, State::Login, Direction::Serverbound, 0x00, true),
        );
    }

    #[test]
    fn protocol_763_uses_login_success_properties() {
        assert_eq!(
            translate_internal_packet_id_for_version(763, State::Login, Direction::Clientbound, 0x02, true),
            crate::protocol::packet::login::clientbound::internal_ids::LoginSuccess_UUID_WithProperties,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Login,
                Direction::Clientbound,
                crate::protocol::packet::login::clientbound::internal_ids::LoginSuccess_UUID_WithProperties,
                false,
            ),
            0x02,
        );
        assert_ne!(
            translate_internal_packet_id_for_version(763, State::Login, Direction::Clientbound, 0x02, true),
            translate_internal_packet_id_for_version(758, State::Login, Direction::Clientbound, 0x02, true),
        );
    }

    #[test]
    fn protocol_763_maps_valence_game_join_boundary() {
        assert_eq!(
            translate_internal_packet_id_for_version(763, State::Play, Direction::Clientbound, 0x28, true),
            crate::protocol::packet::play::clientbound::internal_ids::JoinGame_WorldNames_IsHard_SimDist_LastDeath_PortalCooldown,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Clientbound,
                crate::protocol::packet::play::clientbound::internal_ids::JoinGame_WorldNames_IsHard_SimDist_LastDeath_PortalCooldown,
                false,
            ),
            0x28,
        );
    }

    #[test]
    fn protocol_763_no_longer_treats_play_0x28_as_trade_list() {
        assert_ne!(
            translate_internal_packet_id_for_version(763, State::Play, Direction::Clientbound, 0x28, true),
            translate_internal_packet_id_for_version(758, State::Play, Direction::Clientbound, 0x28, true),
        );
    }

    #[test]
    fn protocol_763_maps_valence_command_tree_boundary() {
        assert_eq!(
            translate_internal_packet_id_for_version(763, State::Play, Direction::Clientbound, 0x10, true),
            crate::protocol::packet::play::clientbound::internal_ids::DeclareCommands,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Clientbound,
                crate::protocol::packet::play::clientbound::internal_ids::DeclareCommands,
                false,
            ),
            0x10,
        );
    }

    #[test]
    fn protocol_763_no_longer_treats_play_0x10_as_clear_titles() {
        assert_ne!(
            translate_internal_packet_id_for_version(763, State::Play, Direction::Clientbound, 0x10, true),
            translate_internal_packet_id_for_version(758, State::Play, Direction::Clientbound, 0x10, true),
        );
    }

    #[test]
    fn protocol_763_maps_valence_game_message_boundary() {
        assert_eq!(
            translate_internal_packet_id_for_version(763, State::Play, Direction::Clientbound, 0x64, true),
            crate::protocol::packet::play::clientbound::internal_ids::ServerMessage_Position,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Clientbound,
                crate::protocol::packet::play::clientbound::internal_ids::ServerMessage_Position,
                false,
            ),
            0x64,
        );
    }

    #[test]
    fn protocol_763_no_longer_treats_play_0x64_as_entity_properties() {
        assert_ne!(
            translate_internal_packet_id_for_version(763, State::Play, Direction::Clientbound, 0x64, true),
            translate_internal_packet_id_for_version(758, State::Play, Direction::Clientbound, 0x64, true),
        );
    }

    #[test]
    fn protocol_763_maps_remaining_observed_valence_boundaries() {
        let boundaries = [
            (0x01, crate::protocol::packet::play::clientbound::internal_ids::SpawnObject_VarInt_HeadYaw),
            (0x02, crate::protocol::packet::play::clientbound::internal_ids::SpawnExperienceOrb),
            (0x03, crate::protocol::packet::play::clientbound::internal_ids::SpawnPlayer_f64_NoMeta),
            (0x04, crate::protocol::packet::play::clientbound::internal_ids::Animation),
            (0x14, crate::protocol::packet::play::clientbound::internal_ids::WindowSetSlot_State),
            (0x1c, crate::protocol::packet::play::clientbound::internal_ids::EntityStatus),
            (0x1e, crate::protocol::packet::play::clientbound::internal_ids::ChunkUnload),
            (0x1f, crate::protocol::packet::play::clientbound::internal_ids::ChangeGameState),
            (0x23, crate::protocol::packet::play::clientbound::internal_ids::KeepAliveClientbound_i64),
            (0x24, crate::protocol::packet::play::clientbound::internal_ids::ChunkData_AndLight_NoTrustEdges),
            (0x2b, crate::protocol::packet::play::clientbound::internal_ids::EntityMove_i16),
            (0x2c, crate::protocol::packet::play::clientbound::internal_ids::EntityLookAndMove_i16),
            (0x2d, crate::protocol::packet::play::clientbound::internal_ids::EntityLook_VarInt),
            (0x2e, crate::protocol::packet::play::clientbound::internal_ids::VehicleTeleport),
            (0x34, crate::protocol::packet::play::clientbound::internal_ids::PlayerAbilities),
            (0x38, crate::protocol::packet::play::clientbound::internal_ids::DeathMessage_VarInt),
            (0x39, crate::protocol::packet::play::clientbound::internal_ids::PlayerRemove_UUIDs),
            (0x3a, crate::protocol::packet::play::clientbound::internal_ids::PlayerInfo_BitSet),
            (0x3c, crate::protocol::packet::play::clientbound::internal_ids::TeleportPlayer_WithConfirm),
            (0x42, crate::protocol::packet::play::clientbound::internal_ids::EntityHeadLook),
            (0x4d, crate::protocol::packet::play::clientbound::internal_ids::SetCurrentHotbarSlot),
            (0x4e, crate::protocol::packet::play::clientbound::internal_ids::UpdateViewPosition),
            (0x4f, crate::protocol::packet::play::clientbound::internal_ids::UpdateViewDistance),
            (0x50, crate::protocol::packet::play::clientbound::internal_ids::SpawnPosition_Angle),
            (0x51, crate::protocol::packet::play::clientbound::internal_ids::ScoreboardDisplay),
            (0x52, crate::protocol::packet::play::clientbound::internal_ids::EntityMetadata),
            (0x53, crate::protocol::packet::play::clientbound::internal_ids::EntityAttach),
            (0x54, crate::protocol::packet::play::clientbound::internal_ids::EntityVelocity),
            (0x55, crate::protocol::packet::play::clientbound::internal_ids::EntityEquipment_Array),
            (0x56, crate::protocol::packet::play::clientbound::internal_ids::SetExperience),
            (0x57, crate::protocol::packet::play::clientbound::internal_ids::UpdateHealth),
            (0x58, crate::protocol::packet::play::clientbound::internal_ids::ScoreboardObjective),
            (0x5b, crate::protocol::packet::play::clientbound::internal_ids::UpdateScore_VarInt),
            (0x64, crate::protocol::packet::play::clientbound::internal_ids::ServerMessage_Position),
            (0x67, crate::protocol::packet::play::clientbound::internal_ids::CollectItem),
            (0x68, crate::protocol::packet::play::clientbound::internal_ids::EntityTeleport_f64),
            (0x69, crate::protocol::packet::play::clientbound::internal_ids::Advancements),
            (0x6a, crate::protocol::packet::play::clientbound::internal_ids::EntityProperties_VarIntVarInt),
            (0x6e, crate::protocol::packet::play::clientbound::internal_ids::Tags_Nested),
        ];

        for (wire_id, internal_id) in boundaries {
            assert_eq!(
                translate_internal_packet_id_for_version(
                    763,
                    State::Play,
                    Direction::Clientbound,
                    wire_id,
                    true,
                ),
                internal_id,
                "wire id 0x{wire_id:02x} should map to the expected Stevenarella internal id",
            );
            assert_eq!(
                translate_internal_packet_id_for_version(
                    763,
                    State::Play,
                    Direction::Clientbound,
                    internal_id,
                    false,
                ),
                wire_id,
                "internal id {internal_id} should map back to wire id 0x{wire_id:02x}",
            );
        }
    }

    #[test]
    fn protocol_763_maps_play_keep_alive_response() {
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Serverbound,
                crate::protocol::packet::play::serverbound::internal_ids::KeepAliveServerbound_i64,
                false,
            ),
            0x12,
        );
    }

    #[test]
    fn protocol_763_maps_play_position_updates() {
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Serverbound,
                crate::protocol::packet::play::serverbound::internal_ids::PlayerPosition,
                false,
            ),
            0x14,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Serverbound,
                0x14,
                true,
            ),
            crate::protocol::packet::play::serverbound::internal_ids::PlayerPosition,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Serverbound,
                crate::protocol::packet::play::serverbound::internal_ids::PlayerPositionLook,
                false,
            ),
            0x15,
        );
        assert_eq!(
            translate_internal_packet_id_for_version(
                763,
                State::Play,
                Direction::Serverbound,
                0x15,
                true,
            ),
            crate::protocol::packet::play::serverbound::internal_ids::PlayerPositionLook,
        );
    }

    #[test]
    fn protocol_763_maps_play_interaction_packets() {
        let boundaries = [
            (
                0x10,
                crate::protocol::packet::play::serverbound::internal_ids::UseEntity_Sneakflag,
            ),
            (
                0x28,
                crate::protocol::packet::play::serverbound::internal_ids::HeldItemChange,
            ),
            (
                0x31,
                crate::protocol::packet::play::serverbound::internal_ids::PlayerBlockPlacement_insideblock_sequence,
            ),
            (
                0x32,
                crate::protocol::packet::play::serverbound::internal_ids::UseItem_WithSequence,
            ),
        ];

        for (wire_id, internal_id) in boundaries {
            assert_eq!(
                translate_internal_packet_id_for_version(
                    763,
                    State::Play,
                    Direction::Serverbound,
                    internal_id,
                    false,
                ),
                wire_id,
            );
            assert_eq!(
                translate_internal_packet_id_for_version(
                    763,
                    State::Play,
                    Direction::Serverbound,
                    wire_id,
                    true,
                ),
                internal_id,
            );
        }
    }

    #[test]
    fn protocol_763_no_longer_uses_758_fallback_for_remaining_observed_boundaries() {
        for wire_id in [0x01, 0x02, 0x03, 0x04, 0x14, 0x1c, 0x1e, 0x1f, 0x24, 0x2b, 0x2c, 0x2d, 0x2e, 0x34, 0x38, 0x39, 0x3a, 0x42, 0x4d, 0x4e, 0x4f, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x5b, 0x67] {
            assert_ne!(
                translate_internal_packet_id_for_version(
                    763,
                    State::Play,
                    Direction::Clientbound,
                    wire_id,
                    true,
                ),
                translate_internal_packet_id_for_version(
                    758,
                    State::Play,
                    Direction::Clientbound,
                    wire_id,
                    true,
                ),
                "wire id 0x{wire_id:02x} should not inherit the protocol 758 mapping",
            );
        }
    }
}
