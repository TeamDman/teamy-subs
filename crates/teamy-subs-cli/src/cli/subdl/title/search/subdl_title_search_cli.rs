use crate::cli::output::CliOutput;
use crate::cli::subdl::api::SubdlClient;
use arbitrary::Arbitrary;
use eyre::Result;
use facet::Facet;
use figue as args;
use teamy_cancellation::CancellationToken;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
pub struct SubdlTitleSearchArgs {
    /// Movie or TV title to find.
    #[facet(args::positional)]
    pub query: String,
}

#[derive(Facet, Debug)]
struct TitleRow {
    name: String,
    year: Option<u16>,
    media_type: Option<String>,
    imdb_id: Option<String>,
    tmdb_id: Option<u64>,
}

impl SubdlTitleSearchArgs {
    /// # Errors
    /// Returns an error if the SubDL title search fails.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        let response = client.search_title(&self.query, cancellation).await?;
        let rows = response
            .results
            .into_iter()
            .map(|title| TitleRow {
                name: title.name,
                year: title.year,
                media_type: title.r#type,
                imdb_id: title.imdb_id,
                tmdb_id: title.tmdb_id,
            })
            .collect::<Vec<_>>();
        Ok(CliOutput::facet(rows))
    }
}
