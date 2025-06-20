use eyre::{ContextCompat, anyhow};
use planus_types::{
    ast::Docstrings,
    intermediate::{AbsolutePath, Declaration, DeclarationKind, Declarations},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Instant,
};

const SCHEMA_DIR: &str = "../flatbuffers-schema/schema";
const OUT_FILE: &str = "./src/planus_flat.rs";

fn get_git_rev(dir: impl AsRef<Path>) -> Option<String> {
    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

pub fn main() -> eyre::Result<()> {
    let start_time = Instant::now();

    if !Path::new(SCHEMA_DIR).exists() {
        Err(anyhow!("Couldn't find flatbuffers schema folder"))?;
    }

    let rlbot_fbs_path = PathBuf::from(SCHEMA_DIR).join("rlbot.fbs");
    let mut declarations = planus_translation::translate_files(&[rlbot_fbs_path.as_path()])
        .context("planus translation failed")?;

    // Replace all links in docstrings with <link>
    for docstring in docstrings_deep_iter_mut(&mut declarations) {
        let mut start = 0;
        while start < docstring.len() {
            let end = {
                let (mut ch_iter, mut i_iter) = (docstring[start..].chars(), start..);
                loop {
                    let next = ch_iter.next();
                    if let Some(ch) = next
                        && !ch.is_whitespace()
                    {
                        i_iter.next();
                        continue;
                    }
                    break i_iter.next().unwrap();
                }
            };
            if let Ok(uri) = fluent_uri::Uri::parse(&docstring[start..end])
                && (uri.scheme().as_str() == "http" || uri.scheme().as_str() == "https")
            {
                docstring.insert(start, '<');
                docstring.insert(end + 1, '>');
            }
            start = end + 1;
        }
    }

    let generated_planus = // No idea why planus renames RLBot to RlBot but this fixes it
        planus_codegen::generate_rust(&declarations)?.replace("RlBot", "RLBot");

    let generated_custom = generate_custom(declarations.declarations.iter().filter(|x| {
        x.0.0
            .last()
            .map(|s| s.ends_with("InterfaceMessage") || s.ends_with("CoreMessage"))
            .unwrap_or(false)
    }))?;

    let now = Instant::now();
    let header = format!(
        "// @generated by build.rs, took {:?}\n\
        pub const RLBOT_FLATBUFFERS_SCHEMA_REV: &str = \"{}\";\n",
        now.duration_since(start_time),
        get_git_rev(SCHEMA_DIR).unwrap_or_else(|| "UNKNOWN".into())
    );

    let raw_out = &[
        header.as_bytes(),
        "////////// CUSTOM GENERATED //////////\n".as_bytes(),
        generated_custom.as_bytes(),
        "////////// PLANUS GENERATED //////////\n".as_bytes(),
        generated_planus.as_bytes(),
    ]
    .concat();

    fs::File::create(OUT_FILE)?.write_all(raw_out)?;

    Ok(())
}

fn docstrings_iter_mut(d: &mut Docstrings) -> impl Iterator<Item = &mut String> {
    d.docstrings.iter_mut().map(|x| &mut x.value)
}

/// Returns an iterator over all docstrings in declarations
fn docstrings_deep_iter_mut(declarations: &mut Declarations) -> impl Iterator<Item = &mut String> {
    // Top 10 most beautiful code OAT.
    declarations
        .declarations
        .iter_mut()
        .map(|(_, decl)| {
            docstrings_iter_mut(&mut decl.docstrings).chain({
                let it: Box<dyn Iterator<Item = &mut String>> = match &mut decl.kind {
                    DeclarationKind::Table(t) => Box::new(
                        t.fields
                            .iter_mut()
                            .map(|(_, field)| docstrings_iter_mut(&mut field.docstrings))
                            .flatten(),
                    ),
                    DeclarationKind::Struct(s) => Box::new(
                        s.fields
                            .iter_mut()
                            .map(|(_, field)| docstrings_iter_mut(&mut field.docstrings))
                            .flatten(),
                    ),
                    DeclarationKind::Enum(e) => Box::new(
                        e.variants
                            .iter_mut()
                            .map(|(_, variant)| docstrings_iter_mut(&mut variant.docstrings))
                            .flatten(),
                    ),
                    DeclarationKind::Union(u) => Box::new(
                        u.variants
                            .iter_mut()
                            .map(|(_, variant)| docstrings_iter_mut(&mut variant.docstrings))
                            .flatten(),
                    ),
                    DeclarationKind::RpcService(_) => unimplemented!("RpcService"),
                };
                it
            })
        })
        .flatten()
}

/// Generate From<EnumVariant> for enum types.
fn generate_custom<'a>(
    enum_decls: impl IntoIterator<Item = (&'a AbsolutePath, &'a Declaration)>,
) -> eyre::Result<String> {
    let mut output = String::new();
    for (decl_path, decl) in enum_decls {
        let DeclarationKind::Union(u) = &decl.kind else {
            return Err(eyre::eyre!("DeclarationKind wasn't union"));
        };
        output.push_str(&format!(
            "// impl From<VARIANT> for {}\n",
            decl_path.0.join("::")
        ));

        for variant in u.variants.keys() {
            let from_t = [&decl_path.0[..decl_path.0.len() - 1], &[variant.clone()]]
                .concat()
                .join("::");
            let for_t = decl_path.0.join("::");
            #[rustfmt::skip]
            output.push_str(&format!(
                "impl From<{from_t}> for {for_t} {{\
                    fn from(value: {from_t}) -> Self {{\
                        Self::{variant}(::std::boxed::Box::new(value))\
                    }}\
                }}\n", // /*{decl_path:#?}*/\n/*{decl:#?}*/
            ));
        }
    }
    Ok(output)
}
