import re

with open('src/commands.rs', 'r', encoding='utf-8') as f:
    code = f.read()

# Replace .as_str().unwrap() or .unwrap_or("...") or unwrap_or(false) etc
import re
# c["name"].as_str().unwrap_or("Unknown") -> crate::conversions::as_str(&c["name"], "name")?
# But wait, we don't have the field name necessarily! We can just use "field".
# Let's write a targeted function to do the replacements carefully.

# Example: contract["name"].as_str().unwrap_or("Unknown") 
# -> crate::conversions::as_str(&contract["name"], "name").unwrap_or_else(|_| "Unknown".to_string())
# Wait, the prompt says "no silent truncation". So we MUST surface errors.
# -> crate::conversions::as_str(&contract["name"], "name")?

code = re.sub(r'(\w+)\["([^"]+)"\]\.as_str\(\)\.unwrap_or\([^)]+\)', r'crate::conversions::as_str(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_bool\(\)\.unwrap_or\([^)]+\)', r'crate::conversions::as_bool(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_u64\(\)\.unwrap_or\([^)]+\)', r'crate::conversions::as_u64(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_i64\(\)\.unwrap_or\([^)]+\)', r'crate::conversions::as_i64(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_f64\(\)\.unwrap_or\([^)]+\)', r'crate::conversions::as_f64(&\1["\2"], "\2")?', code)

# For unwrap()
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_str\(\)\.unwrap\(\)', r'crate::conversions::as_str(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_bool\(\)\.unwrap\(\)', r'crate::conversions::as_bool(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_u64\(\)\.unwrap\(\)', r'crate::conversions::as_u64(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_i64\(\)\.unwrap\(\)', r'crate::conversions::as_i64(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_f64\(\)\.unwrap\(\)', r'crate::conversions::as_f64(&\1["\2"], "\2")?', code)
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_array\(\)\.unwrap\(\)', r'crate::conversions::as_array(&\1["\2"], "\2")?', code)

# Custom edits that are tricky:
# `.as_str().ok_or_else(|| anyhow::anyhow!(...))?` -> maybe leave alone?
# Or `unwrap_or_default()`
code = re.sub(r'(\w+)\["([^"]+)"\]\.as_str\(\)\.unwrap_or_default\(\)', r'crate::conversions::as_str(&\1["\2"], "\2")?', code)

# We also need to fix `map(|c| serde_json::json!({...}))` to handle `?`. 
# Inside map, `?` won't work unless we return `Result` and `.collect::<Result<Vec<_>, _>>()?`.
# Let's look for `.map(|c| serde_json::json!(`
code = code.replace(".map(|c| serde_json::json!({\n", ".map(|c| -> Result<_> { Ok(serde_json::json!({\n")
code = code.replace("            }))\n            .collect();", "            })) })\n            .collect::<Result<_, _>>()?;")
code = code.replace("            }))\n            .collect()", "            })) })\n            .collect::<Result<_, _>>()?")

with open('src/commands.rs', 'w', encoding='utf-8') as f:
    f.write(code)

print("Done")
