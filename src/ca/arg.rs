use clap::{Args, Parser, Subcommand};

use super::{local_state::LocalCAState, operation::SCPCAOperation};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct CACli {
    #[command(subcommand)]
    command: CACmd,
}

#[derive(Subcommand, Debug)]
enum CACmd {
    CreateNamespace(CreateNamespaceArg),
}

#[derive(Args, Debug)]
struct CreateNamespaceArg {
    namespace: String,
}

impl From<String> for CACli {
    fn from(s: String) -> Self {
        let mut cli = vec![""];
        cli.extend(s.split_whitespace());
        let arg = CACli::try_parse_from(cli);
        arg.unwrap()
    }
}

impl CACmd {
    pub fn to_scp_operation(self, local_state: &LocalCAState) -> Option<SCPCAOperation> {
        match self {
            CACmd::CreateNamespace(arg) => {
                let operation = local_state.create_name_space(&arg.namespace).ok()?;
                let scp_operation = SCPCAOperation(vec![operation]);
                Some(scp_operation)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use clap::Parser;

    use crate::ca::arg::{CACmd, CreateNamespaceArg};

    use super::CACli;

    #[test]
    fn create_new_namespace_cmd_ok() {
        let cmd = CACmd::CreateNamespace(CreateNamespaceArg {
            namespace: "namespace1".to_string(),
        });
        let local_state = crate::ca::local_state::LocalCAState::init_state_from_pkcs8_pem(
            crate::ca::crypto::TEST_OPENSSL_PRIVATE_KEY,
        );

        let scp_operation = cmd.to_scp_operation(&local_state).unwrap();

        assert_eq!(scp_operation.0.len(), 1);
        match &scp_operation.0[0] {
            crate::ca::operation::CAOperation::SetRoot(set_root_operation) => {
                assert_eq!(
                    set_root_operation.entry.application_identifier,
                    "namespace1"
                );
            }
            _ => panic!("not reached"),
        }
    }

    #[test]
    fn parse_cli() {
        // https://stackoverflow.com/questions/74465951/how-to-parse-custom-string-with-clap-derive
        let cli = vec!["", "create-namespace", "namespace1"];
        let arg = CACli::parse_from(cli);
        match arg.command {
            CACmd::CreateNamespace(create_namespace_arg) => {
                assert_eq!(create_namespace_arg.namespace, "namespace1");
            }
            _ => panic!("not reached"),
        }

        let cli_str = "create-namespace namespace1";
        let arg = CACli::from(cli_str.to_string());

        match arg.command {
            CACmd::CreateNamespace(create_namespace_arg) => {
                assert_eq!(create_namespace_arg.namespace, "namespace1");
            }
            _ => panic!("not reached"),
        }
    }
}
