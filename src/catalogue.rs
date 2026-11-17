use crate::system::{PLANET_COUNT, STAR_COUNT};

pub struct PlanetInfo {
    pub description: &'static str,
    pub kind: &'static str,
    pub inhabitants: &'static str,
    pub climate: &'static str,
    pub film: &'static str,
    pub scene: &'static str,
}

const PLANETS: [PlanetInfo; PLANET_COUNT] = [
    PlanetInfo {
        description:
            "Desierto del Borde Exterior con granjas de humedad y puertos como Mos Eisley.",
        kind: "Rocoso desértico",
        inhabitants: "Humanos, jawas y tusken",
        climate: "Árido y caluroso",
        film: "Una nueva esperanza",
        scene: "Luke contempla la puesta de los dos soles cerca de la granja Lars.",
    },
    PlanetInfo {
        description: "Un mundo minero de roca oscura, volcanes y ríos de lava.",
        kind: "Rocoso volcánico",
        inhabitants: "Mustafarianos",
        climate: "Calor extremo",
        film: "La venganza de los Sith",
        scene: "Obi-Wan y Anakin se baten en duelo sobre la lava.",
    },
    PlanetInfo {
        description: "Archipiélagos tropicales rodean un complejo militar del Imperio.",
        kind: "Rocoso oceánico",
        inhabitants: "Guarnición imperial",
        climate: "Tropical",
        film: "Rogue One",
        scene: "Jyn Erso transmite los planos de la Estrella de la Muerte a la flota rebelde.",
    },
    PlanetInfo {
        description: "Llanuras verdes, lagos pantanosos y ciudades fluviales como Theed.",
        kind: "Rocoso templado",
        inhabitants: "Humanos de Naboo y gungans",
        climate: "Templado",
        film: "La amenaza fantasma",
        scene: "Qui-Gon y Obi-Wan enfrentan a Darth Maul en el complejo generador de Theed.",
    },
    PlanetInfo {
        description: "Los wookiees construyen sus hogares entre las copas de los bosques.",
        kind: "Rocoso boscoso",
        inhabitants: "Wookiees",
        climate: "Templado",
        film: "La venganza de los Sith",
        scene: "Yoda sobrevive a la Orden 66 y escapa con ayuda de Chewbacca y Tarfful.",
    },
    PlanetInfo {
        description: "Ciudades sobre pilotes se elevan por encima de un océano tormentoso.",
        kind: "Rocoso oceánico",
        inhabitants: "Kaminoanos",
        climate: "Tormentas frecuentes",
        film: "El ataque de los clones",
        scene: "Obi-Wan pelea con Jango Fett bajo la lluvia en una plataforma de Ciudad Tipoca.",
    },
    PlanetInfo {
        description: "La Ciudad de las Nubes flota en una franja habitable de su atmósfera.",
        kind: "Gigante gaseoso",
        inhabitants: "Ugnaughts y colonos",
        climate: "Nuboso, con corrientes intensas",
        film: "El Imperio contraataca",
        scene: "Darth Vader revela a Luke que es su padre en la Ciudad de las Nubes.",
    },
    PlanetInfo {
        description: "Glaciares y llanuras heladas rodean la antigua base rebelde Eco.",
        kind: "Rocoso glacial",
        inhabitants: "Wampas, tauntauns y guarnición rebelde",
        climate: "Polar",
        film: "El Imperio contraataca",
        scene: "Los AT-AT imperiales avanzan sobre la base Eco durante la evacuación.",
    },
];

pub fn planet(index: usize) -> Option<&'static PlanetInfo> {
    PLANETS.get(index.checked_sub(STAR_COUNT)?)
}
