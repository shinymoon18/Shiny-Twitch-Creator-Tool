use crate::{media::{self, Audio, Result}, video};

#[derive(Clone)]
pub struct Measurement { pub loudness: Option<f64>, pub peak: Option<f64> }

pub fn measure(audio: &Audio, start: f64, length: f64, options: Option<&video::Options>, gain: f32) -> Result<Measurement> {
    let mut cmd = media::command("ffmpeg");
    cmd.args(["-hide_banner", "-nostdin", "-ss", &start.to_string(), "-i"]).arg(&audio.path);
    let meter = "loudnorm=I=-18:TP=-1:LRA=11:print_format=json";
    if let (Some(info), Some(options)) = (&audio.video, options) {
        let graph = video::audio_graph(info, options, length)?.ok_or("Selecciona Mantener original o un archivo local para medir sonido.")?;
        video::add_external_input(&mut cmd, options);
        cmd.args(["-filter_complex", &format!("{graph};[mixed]{meter}[measured]"), "-map", "[measured]"]);
    } else { cmd.args(["-map", "0:a:0", "-af", &format!("atrim=duration={length:.6},volume={gain:.3}dB,{meter}")]); }
    let (_, report) = media::run_report(cmd.args(["-t", &length.to_string(), "-vn", "-f", "null", "-"]), 7200)?;
    let begin = report.rfind('{').ok_or("No se pudo medir el volumen de este fragmento.")?;
    let end = report[begin..].find('}').map(|i| begin + i + 1).ok_or("Medición incompleta.")?;
    let value: serde_json::Value = serde_json::from_str(&report[begin..end]).map_err(|e| e.to_string())?;
    let number = |key: &str| value[key].as_str().and_then(|s| s.parse::<f64>().ok()).filter(|v| v.is_finite());
    Ok(Measurement { loudness: number("input_i"), peak: number("input_tp") })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn manual_gain_changes_measured_alert_loudness_and_export() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("alerta.wav");
        media::run(media::command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "sine=frequency=880:duration=4", "-af", "volume=0.25"]).arg(&path), 20).unwrap();
        let audio = media::load(path).unwrap();
        let original = measure(&audio, 0.0, 4.0, None, 0.0).unwrap();
        let adjusted = measure(&audio, 0.0, 4.0, None, 6.0).unwrap();
        assert!((adjusted.loudness.unwrap() - original.loudness.unwrap() - 6.0).abs() < 0.3);
        assert!((adjusted.peak.unwrap() - original.peak.unwrap() - 6.0).abs() < 0.3);
        let path = dir.path().join("ajustada.wav"); media::export_gain(&audio, 0.0, 4.0, &path, 6.0).unwrap();
        let result = measure(&media::load(path).unwrap(), 0.0, 4.0, None, 0.0).unwrap();
        assert!((result.loudness.unwrap() - adjusted.loudness.unwrap()).abs() < 0.3);
    }
}
