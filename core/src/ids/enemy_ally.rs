use crate::int_enum;

int_enum! {
    pub enum EnemyAlly: u8 {
        Anyone = 0, //Includes all allegiances.
        Inanimate = 1, //E.g. Sun Statue in Temple of Amaunator ("rngsta01.cre")
        Pc = 2, // Regular party members.
        Familiar = 3, // Familiars of mages.
        Ally = 4,
        Controlled = 5, // Creatures fully under control of the player.
        Charmed = 6, // Uncontrolled ally (green selection circle) of the player.
        Reallycharmed = 7, // Creatures fully under control of the player.
        GoodButRed = 28, // Creatures of same allegiance as party, but uses red (hostile) selection circles. Can not be controlled by the player.
        GoodButBlue = 29, // Creatures of same allegiance as party, but uses blue (neutral) selection circles. Can not be controlled by the player.
        Goodcutoff = 30, // Used by script actions and triggers. Includes all party-friendly allegiances.
        Notgood = 31, // Used by script actions and triggers. Includes everything except party-friendly allegiances.
        Anything = 126,
        AreaObject = 127, // Doors, Containers, Regions and Animations. It is included in EA groups NOTGOOD, ANYTHING, and NOTEVIL.
        Neutral = 128,
        NotNeutral = 198, // Used by neutrals when targetting with enemy-only spells.
        NotEvil = 199, // Used by script actions and triggers. Includes everything except hostile allegiances.
        EvilCutoff = 200, // Used by script actions and triggers. Includes all hostile allegiances.
        EvilButGreen = 201, //Hostile creatures, but uses green (friendly) selection circles.
        EvilButBlue = 202, //Hostile creatures, but uses blue (neutral) selection circles.
        CharmedPc = 254, //This is just a separate EA from ENEMY for detection purposes. They're still valid objects for EVILCUTOFF and NearestEnemyOf(), but not by ENEMY. It's not specific to PCs.
        Enemy = 255, // Creatures that are hostile to the party and allied creatures.
    }
}
