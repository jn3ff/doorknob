use std::{collections::HashMap, sync::Arc};

use axum::{
    Form,
    extract::{Query, State},
    response::{Html, Redirect},
};
use serde::Deserialize;
use tokio::sync::mpsc::Sender;

use crate::{
    auth::verify_password,
    lock::{InstructionSource, LockInstruction, LockInstructor},
};

#[derive(Deserialize)]
pub struct LockRequest {
    pub passcode: String,
    pub action: String,
}

fn format_err_message(message: &str) -> String {
    let mut s = String::from("<p style='color: red;'>");
    s.push_str(message);
    s.push_str("</p>");
    s
}

pub async fn home(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    let error_msg = if let Some(error) = params.get("error") {
        match error.as_str() {
            "invalid_password" => format_err_message("Invalid password. Please try again."),
            "in_use" => format_err_message("Lock is in use. Please try again later."),
            "internal_error" => format_err_message(
                "Internal service issue. Please try again. Service may need to be restarted",
            ),
            "wtf_was_that" => {
                let mut s = String::new();
                for _ in 0..10000 {
                    s.push_str("<p style='width: 100vw; color: darkred; font-size: 72px; font-weight: 900; text-transform: uppercase; text-shadow: 4px 4px 8px rgba(0,0,0,0.6); font-family: Impact, sans-serif; letter-spacing: 2px; text-align: left;'>GET OUT OF MY HOUSE</p>")
                }
                s
            }
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let success_msg = match params.get("success") {
        Some(_) => "<p style='color: green;'>Success!</p>",
        _ => "",
    };

    Html(format!(
        r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Door Control</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <style>
                    body {{
                        font-family: Arial, sans-serif;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                        margin: 0;
                        background-color: #f5f5f5;
                    }}
                    .container {{
                        background: white;
                        padding: 20px;
                        border-radius: 10px;
                        box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
                        width: 300px;
                        text-align: center;
                    }}
                    input {{
                        width: 100%;
                        padding: 12px;
                        margin-bottom: 15px;
                        border: 1px solid #ddd;
                        border-radius: 4px;
                        box-sizing: border-box;
                        font-size: 18px;
                    }}
                    button {{
                        width: 100%;
                        padding: 15px;
                        color: white;
                        border: none;
                        border-radius: 4px;
                        cursor: pointer;
                        font-size: 18px;
                    }}
                    button.unlock {{
                        background-color: #4CAF50;
                    }}
                    button.lock {{
                        margin-top: 10px;
                        background-color: #222222;
                    }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h2>Door Control</h2>
                    {error_msg}
                    {success_msg}
                    <form action="/door-control" method="post">
                        <input type="password" name="passcode" placeholder="Enter Passcode" required>
                        <button class="unlock" type="submit" name="action" value="unlock">Unlock Door</button>
                        <button class="lock" type="submit" name="action" value="lock">Lock Door</button>
                    </form>
                </div>
            </body>
            </html>
        "#,
    ))
}

pub async fn door_control(
    State(lock_tx): State<Arc<Sender<LockInstruction>>>,
    Form(form): Form<LockRequest>,
) -> Redirect {
    let instruction = match form.action.as_str() {
        "lock" => Some(LockInstruction::EnsureLocked(InstructionSource::Api)),
        "unlock" => Some(LockInstruction::EnsureUnlocked(InstructionSource::Api)),
        _ => None,
    };
    if instruction.is_none() {
        return Redirect::to("/home?error=wtf_was_that");
    }
    match verify_password(form.passcode.as_str()).await {
        Ok(true) => {
            if let Err(e) = lock_tx.send_instruction(instruction.expect("already checked none")) {
                println!("Dropping instruction. {}", e);
                return Redirect::to("/home?error=in_use");
            }

            Redirect::to("/home?success")
        }
        Ok(false) => {
            println!("Bad password entered");
            Redirect::to("/home?error=invalid_password")
        }
        Err(e) => {
            eprintln!("argon issue with hashed password {:?}", e);
            Redirect::to("/home?error=internal_error")
        }
    }
}
