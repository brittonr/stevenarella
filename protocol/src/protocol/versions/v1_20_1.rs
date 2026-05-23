use crate::protocol::{packet, Direction, State};

const PLAY_CLIENTBOUND_OVERRIDES: &[(i32, i32)] = &[
    (0x01, packet::play::clientbound::internal_ids::SpawnObject_VarInt_HeadYaw),
    (0x02, packet::play::clientbound::internal_ids::SpawnExperienceOrb),
    (0x03, packet::play::clientbound::internal_ids::SpawnPlayer_f64_NoMeta),
    (0x04, packet::play::clientbound::internal_ids::Animation),
    (0x10, packet::play::clientbound::internal_ids::DeclareCommands),
    (0x14, packet::play::clientbound::internal_ids::WindowSetSlot_State),
    (0x1c, packet::play::clientbound::internal_ids::EntityStatus),
    (0x1e, packet::play::clientbound::internal_ids::ChunkUnload),
    (0x1f, packet::play::clientbound::internal_ids::ChangeGameState),
    (0x23, packet::play::clientbound::internal_ids::KeepAliveClientbound_i64),
    (0x24, packet::play::clientbound::internal_ids::ChunkData_AndLight_NoTrustEdges),
    (
        0x28,
        packet::play::clientbound::internal_ids::JoinGame_WorldNames_IsHard_SimDist_LastDeath_PortalCooldown,
    ),
    (0x2b, packet::play::clientbound::internal_ids::EntityMove_i16),
    (0x2c, packet::play::clientbound::internal_ids::EntityLookAndMove_i16),
    (0x2d, packet::play::clientbound::internal_ids::EntityLook_VarInt),
    (0x2e, packet::play::clientbound::internal_ids::VehicleTeleport),
    (0x34, packet::play::clientbound::internal_ids::PlayerAbilities),
    (0x38, packet::play::clientbound::internal_ids::DeathMessage_VarInt),
    (0x39, packet::play::clientbound::internal_ids::PlayerRemove_UUIDs),
    (0x3a, packet::play::clientbound::internal_ids::PlayerInfo_BitSet),
    (0x3c, packet::play::clientbound::internal_ids::TeleportPlayer_WithConfirm),
    (0x42, packet::play::clientbound::internal_ids::EntityHeadLook),
    (0x4d, packet::play::clientbound::internal_ids::SetCurrentHotbarSlot),
    (0x4e, packet::play::clientbound::internal_ids::UpdateViewPosition),
    (0x4f, packet::play::clientbound::internal_ids::UpdateViewDistance),
    (0x50, packet::play::clientbound::internal_ids::SpawnPosition_Angle),
    (0x51, packet::play::clientbound::internal_ids::ScoreboardDisplay),
    (0x52, packet::play::clientbound::internal_ids::EntityMetadata),
    (0x53, packet::play::clientbound::internal_ids::EntityAttach),
    (0x54, packet::play::clientbound::internal_ids::EntityVelocity),
    (0x55, packet::play::clientbound::internal_ids::EntityEquipment_Array),
    (0x56, packet::play::clientbound::internal_ids::SetExperience),
    (0x57, packet::play::clientbound::internal_ids::UpdateHealth),
    (0x58, packet::play::clientbound::internal_ids::ScoreboardObjective),
    (0x5b, packet::play::clientbound::internal_ids::UpdateScore_VarInt),
    (0x64, packet::play::clientbound::internal_ids::ServerMessage_Position),
    (0x67, packet::play::clientbound::internal_ids::CollectItem),
    (0x68, packet::play::clientbound::internal_ids::EntityTeleport_f64),
    (0x69, packet::play::clientbound::internal_ids::Advancements),
    (
        0x6a,
        packet::play::clientbound::internal_ids::EntityProperties_VarIntVarInt,
    ),
    (0x6e, packet::play::clientbound::internal_ids::Tags_Nested),
];

const PLAY_SERVERBOUND_OVERRIDES: &[(i32, i32)] = &[
    (0x10, packet::play::serverbound::internal_ids::UseEntity_Sneakflag),
    (0x12, packet::play::serverbound::internal_ids::KeepAliveServerbound_i64),
    (0x14, packet::play::serverbound::internal_ids::PlayerPosition),
    (0x15, packet::play::serverbound::internal_ids::PlayerPositionLook),
    (0x28, packet::play::serverbound::internal_ids::HeldItemChange),
    (
        0x31,
        packet::play::serverbound::internal_ids::PlayerBlockPlacement_insideblock_sequence,
    ),
    (0x32, packet::play::serverbound::internal_ids::UseItem_WithSequence),
];

const LOGIN_SERVERBOUND_OVERRIDES: &[(i32, i32)] = &[(
    0x00,
    packet::login::serverbound::internal_ids::LoginStart_WithOptionalUuid,
)];

const LOGIN_CLIENTBOUND_OVERRIDES: &[(i32, i32)] = &[(
    0x02,
    packet::login::clientbound::internal_ids::LoginSuccess_UUID_WithProperties,
)];

pub fn translate_internal_packet_id(state: State, dir: Direction, id: i32, to_internal: bool) -> i32 {
    if state == State::Login && dir == Direction::Clientbound {
        if to_internal {
            if let Some((_, internal_id)) = LOGIN_CLIENTBOUND_OVERRIDES
                .iter()
                .find(|(wire_id, _)| *wire_id == id)
            {
                return *internal_id;
            }
        } else if let Some((wire_id, _)) = LOGIN_CLIENTBOUND_OVERRIDES
            .iter()
            .find(|(_, internal_id)| *internal_id == id)
        {
            return *wire_id;
        }
    }

    if state == State::Login && dir == Direction::Serverbound {
        if to_internal {
            if let Some((_, internal_id)) = LOGIN_SERVERBOUND_OVERRIDES
                .iter()
                .find(|(wire_id, _)| *wire_id == id)
            {
                return *internal_id;
            }
        } else if let Some((wire_id, _)) = LOGIN_SERVERBOUND_OVERRIDES
            .iter()
            .find(|(_, internal_id)| *internal_id == id)
        {
            return *wire_id;
        }
    }

    if state == State::Play && dir == Direction::Clientbound {
        if to_internal {
            if let Some((_, internal_id)) = PLAY_CLIENTBOUND_OVERRIDES
                .iter()
                .find(|(wire_id, _)| *wire_id == id)
            {
                return *internal_id;
            }
        } else if let Some((wire_id, _)) = PLAY_CLIENTBOUND_OVERRIDES
            .iter()
            .find(|(_, internal_id)| *internal_id == id)
        {
            return *wire_id;
        }
    }

    if state == State::Play && dir == Direction::Serverbound {
        if to_internal {
            if let Some((_, internal_id)) = PLAY_SERVERBOUND_OVERRIDES
                .iter()
                .find(|(wire_id, _)| *wire_id == id)
            {
                return *internal_id;
            }
        } else if let Some((wire_id, _)) = PLAY_SERVERBOUND_OVERRIDES
            .iter()
            .find(|(_, internal_id)| *internal_id == id)
        {
            return *wire_id;
        }
    }

    super::v1_18_2::translate_internal_packet_id(state, dir, id, to_internal)
}
