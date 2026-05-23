use crate::protocol::{packet, Direction, State};

pub fn translate_internal_packet_id(state: State, dir: Direction, id: i32, to_internal: bool) -> i32 {
    if state == State::Play && dir == Direction::Clientbound {
        if to_internal && id == 0x10 {
            return packet::play::clientbound::internal_ids::DeclareCommands;
        }

        if !to_internal && id == packet::play::clientbound::internal_ids::DeclareCommands {
            return 0x10;
        }

        if to_internal && id == 0x28 {
            return packet::play::clientbound::internal_ids::JoinGame_WorldNames_IsHard_SimDist;
        }

        if !to_internal && id == packet::play::clientbound::internal_ids::JoinGame_WorldNames_IsHard_SimDist {
            return 0x28;
        }
    }

    super::v1_18_2::translate_internal_packet_id(state, dir, id, to_internal)
}
