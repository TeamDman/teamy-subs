use crate::cli::output::CliOutput;
use crate::cli::subdl::api::FileSearchResponse;
use crate::cli::subdl::api::SubdlClient;
use arbitrary::Arbitrary;
use eyre::Result;
use eyre::bail;
use facet::Facet;
use figue as args;
use std::path::Path;
use teamy_cancellation::CancellationToken;

#[derive(Facet, Arbitrary, Debug, PartialEq)]
#[facet(rename_all = "kebab-case")]
pub struct SubdlSubtitleSearchArgs {
    /// Local video file or release filename to match.
    #[facet(args::named)]
    pub file: String,

    /// Subtitle language code, such as en or fr. Defaults to en.
    #[facet(args::named)]
    pub language: Option<String>,
}

#[derive(Facet, Debug)]
struct SubtitleRow {
    pick: usize,
    title: String,
    year: Option<u16>,
    release_name: String,
    language: String,
    match_score: Option<f64>,
    archive_name: String,
}

impl SubdlSubtitleSearchArgs {
    /// # Errors
    /// Returns an error if SubDL cannot match the filename.
    pub async fn invoke(
        self,
        client: &SubdlClient,
        cancellation: &CancellationToken,
    ) -> Result<CliOutput> {
        let filename = Path::new(&self.file)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| eyre::eyre!("--file must have a UTF-8 filename"))?;
        let language = self.language.as_deref().unwrap_or("en");
        let response = client.search_file(filename, language, cancellation).await?;
        if !response.status {
            bail!("SubDL did not return a successful filename match");
        }
        Ok(CliOutput::facet(summarize(response)))
    }
}

fn summarize(response: FileSearchResponse) -> Vec<SubtitleRow> {
    let title = response.title_match.as_ref().map_or_else(
        || {
            response
                .results
                .first()
                .map_or_else(String::new, |item| item.name.clone())
        },
        |item| item.title.clone(),
    );
    let year = response.title_match.as_ref().and_then(|item| item.year);
    response
        .subtitles
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| SubtitleRow {
            pick: index + 1,
            title: title.clone(),
            year,
            release_name: candidate.release_name,
            language: candidate.language.unwrap_or(candidate.lang),
            match_score: candidate.match_score,
            archive_name: candidate.name,
        })
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_output_never_contains_download_url_or_key() {
        let response: FileSearchResponse = facet_json::from_str(
            r#"{
            "status": true,
            "match": { "title": "Perfect Blue", "year": 1997 },
            "subtitles": [{
                "name": "match.zip",
                "release_name": "Perfect.Blue.1997.1080p.BluRay.x264-[YTS.AM]",
                "lang": "english", "language": "EN", "match_score": 1.0,
                "url": "/subtitle/1-2.zip?api_key=dummy-secret"
            }]
        }"#,
        )
        .expect("fixture should match the observed API shape");
        let output = facet_json::to_string_pretty(&summarize(response)).unwrap();
        assert!(output.contains("Perfect Blue"));
        assert!(!output.contains("dummy-secret"));
        assert!(!output.contains("/subtitle/"));
    }
}
