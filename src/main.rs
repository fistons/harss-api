/// Source of articles, over da web
struct Flux {
    url: String,
    title: String,
}

/// # Basic **article** 
struct Article {
    url: String,
    title: String,
    content: String,
}

/// # Abstraction over a collection of Flux, because, one day, we will store them somewhere
struct FluxStorage<'a> {
    flux: &'a mut Vec<Flux>
}

impl<'a> FluxStorage<'a> {
    fn add_flux(&mut self, new_flux: Flux) {
        self.flux.push(new_flux) // 🖕
    }

    fn get(&self, index: usize) -> Option<&Flux> {
        self.flux.get(index)
    }

    fn get_urls(&self) -> Vec<&String> {
        self.flux.iter().map(|f| &f.url).collect()
    }
}

fn main() {}


#[cfg(test)]
mod test {
    use crate::{Flux, FluxStorage};

    #[test]
    fn test() {
        let mut collection = Vec::<Flux>::new();
        let mut store = FluxStorage { flux: &mut collection };

        let flux_1 = Flux {
            title: String::from("CanardPc"),
            url: String::from("https://canardpc.com/rss.xml"),
        };

        let flux_2 = Flux {
            title: String::from("El mundo en français"),
            url: String::from("https://www.lemonde.fr/rss/une.xml"),
        };


        store.add_flux(flux_1);
        store.add_flux(flux_2);

        assert_eq!(store.get(0).unwrap().title, String::from("CanardPc"));
        assert_eq!(store.get(1).unwrap().title, String::from("El mundo en français"));

        assert_eq!(*store.get_urls().get(0).unwrap(), &String::from("https://canardpc.com/rss.xml"))
        //🖕
    }
}
