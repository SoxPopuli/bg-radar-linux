use core::{
    error::Error,
    find_game_process, get_static_entity_list,
    types::{CGameAIBase, CGameSprite, ObjectType},
};

fn main() -> Result<(), Error> {
    let game_process = find_game_process(true)?;
    let entities = get_static_entity_list(&game_process)?;

    entities
        .into_iter()
        .filter(|x| x.id != u16::MAX)
        .map(|x| {
            let base = CGameAIBase::new(&game_process, &x);

            base.map(|base| (x, base))
        })
        .filter_map(|x| {
            if let Ok((entity, Some(base))) = x
                && base.object.object_type == ObjectType::Sprite
            {
                CGameSprite::new(&game_process, &entity, base).unwrap()
            } else {
                None
            }
        })
        .for_each(|x| println!("{x:#?}"));

    Ok(())
}
