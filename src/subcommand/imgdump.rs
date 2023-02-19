use std::io::Write;

use {
    super::*,
    media::Media,
    std::path::Path,
};

#[derive(Debug, Parser)]
pub(crate) struct ImgDump {
    #[clap(long, default_value="images", help="Directory to put images.")]
    image_dir: String,

    #[clap(long, default_value="0", help="Inscription number to start with")]
    start: u64,

    #[clap(long, default_value="0", help="Inscription number to end with")]
    end: u64,
}

impl ImgDump {
  pub(crate) fn run(self, options: Options) -> Result {
    // open the index and update it
    let index = Index::open(&options)?;
    index.update()?;
    let context = new_context(index, self.image_dir);

    // iterate over the specified range of inscriptions.
    context.process_inscriptions(self.start, self.end)
  }
}

struct Context {
  index: Index,
  image_dir: String,
}
  
struct Stats {
  images_count: u64,
  images_bytes: usize,
  images_skipped: u64,
}

fn new_context(index: Index, image_dir: String) -> Context {
  Context {index, image_dir}
}

impl Context {

  fn process_inscriptions(self, start: u64, end: u64) -> Result {
    let mut stats = Stats {images_count: 0, images_bytes: 0, images_skipped: 0};
    let mut from = Some(start);
    loop {
        let (
          inscriptions,
          _,
          next
        ) = self.index.get_latest_inscriptions_with_prev_and_next(100, from)?;
        match next {
            None => break,
            Some(next) => {
              if end > 0 && next > end {
                break
              }
              from = Some(next)
            }
        }
        let inscriptions_len = inscriptions.len();
        for i in inscriptions {
          self.process(&i, &mut stats)?;
        }
        println!("From: {}, Got: {}", from.unwrap_or(0), inscriptions_len)
    }
    println!("Images: {} total ({} skipped) {} bytes", stats.images_count, stats.images_skipped, stats.images_bytes);
    Ok(()) 
  }

  fn process(&self, inscription_id: &InscriptionId, stats: &mut Stats) -> Result {
    if let Some(entry) = self.index.get_inscription_entry(*inscription_id)? {
      if let Some(inscription) = self.index.get_inscription_by_id(*inscription_id)? {

        let content_length = inscription.content_length().unwrap_or(0);
        let content_type = inscription.content_type().unwrap_or("");

        if !content_type.starts_with("image/"){
          return Ok(())
        }

        // calculate the file path
        let extension = Media::extension_for_content_type(content_type).unwrap_or("unk");
        let filename = format!("{:0>8}.{}", entry.number, extension);
        let path = Path::new(&self.image_dir);
        if !path.exists(){
          // make dir if it doesn't exist
          std::fs::create_dir_all(path)?;
        }
        let file_path = path.join(filename);
        // if the file exists, check the size, if it is as expected, skip
        if file_path.exists() {
          let metadata = fs::metadata(&file_path)?;
          if metadata.len() == content_length as u64 {
            stats.images_skipped += 1;
            return Ok(())
          }
        }
        
        // if we reach here, write the file to disk
        let mut file = std::fs::File::create(file_path)?;
        if let Some(body) = inscription.body() {
          file.write_all(body)?;
        }

        stats.images_count += 1;
        stats.images_bytes += content_length; 
      }
    };
    Ok(())
  }
}
