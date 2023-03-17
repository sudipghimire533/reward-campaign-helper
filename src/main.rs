const NODE_CONNECT: &'static str = "ws://127.0.0.1:9988";

#[subxt::subxt(runtime_metadata_path = "chain-metadata.scale")]
pub mod datahighway {}

use datahighway::runtime_types::pallet_reward_campaign::types as campaign_types;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sp_core::sr25519::Pair as Sr25519Pair;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use std::error::Error as StdError;
use std::fs::File;
use std::io::BufReader;
use std::option;
use std::path::PathBuf;
use subxt::blocks::ExtrinsicEvents;
use subxt::config::Config;
use subxt::ext::sp_runtime::traits::Zero;
use subxt::tx::TxPayload;
use subxt::{tx, OnlineClient, PolkadotConfig, SubstrateConfig};

type DatahighwayOnlineClient = subxt::client::OnlineClient<DatahighwayConfig>;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct DatahighwayConfig;

type AccountId = subxt::utils::AccountId32;
type BlockNumber = u32;
type Balance = u128;
type Index = u32;
impl Config for DatahighwayConfig {
    type Index = Index;
    type Hash = subxt::ext::sp_core::H256;
    type AccountId = AccountId;
    type Address = subxt::ext::sp_runtime::MultiAddress<AccountId, Index>;
    type Hasher = <SubstrateConfig as Config>::Hasher;
    type Header = subxt::config::substrate::SubstrateHeader<BlockNumber, Self::Hasher>;
    type Signature = subxt::ext::sp_runtime::MultiSignature;
    type ExtrinsicParams = <SubstrateConfig as Config>::ExtrinsicParams;
}

type PairSigner = tx::PairSigner<DatahighwayConfig, Sr25519Pair>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // read the input file
    let input_file_name = std::env::args()
        .next()
        .unwrap_or("reward-campaign.json".to_string());
    let input_file = File::open(input_file_name).unwrap();
    let input_file_reader = BufReader::new(input_file);
    let input = serde_json::from_reader::<_, InputFile>(input_file_reader).unwrap();
    let campaign = input.process().unwrap();

    // this is the campaign_id
    let campaign_id = campaign.campaign_id;

    // Start this campaign
    start_campaign(
        campaign_id,
        CreateCampaignParams {
            hoster: Some(signer().account_id().to_owned()),
            starts_from: Some(campaign.starts_from),
            end_target: campaign.ends_at,
            instant_percentage: campaign_types::SmallRational {
                numenator: campaign.instant_percentage.0,
                denomator: campaign.instant_percentage.1,
            },
        },
    )
    .await
    .unwrap();

    // add contributers
    for contributer in campaign.contributers {
        let amount = contributer.reward_amount();
        let contributer = contributer.who;

        if let Err(err) = add_contributer(campaign_id, contributer.clone(), amount).await {
            eprintln!("Cannot add contributer: {contributer:?}. Error: {:?}", err);
            eprintln!("--------------");
        }
    }

    Ok(())
}

async fn start_campaign(
    campaign_id: CampaignId,
    campaign_info: CreateCampaignParams,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .start_new_campaign(campaign_id, campaign_info);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn update_campaign(
    campaign_id: CampaignId,
    new_campaign_info: UpdateCampaignParams,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .update_campaign(campaign_id, new_campaign_info);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn discard_campaign(campaign_id: CampaignId) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().discard_campaign(campaign_id);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn wipe_campaign(campaign_id: CampaignId) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().wipe_campaign(campaign_id);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn add_contributer(
    campaign_id: CampaignId,
    contributer: AccountId,
    amount: Balance,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .add_contributer(campaign_id, contributer, amount);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn remove_contributer(
    campaign_id: CampaignId,
    contributer: AccountId,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .remove_contributer(campaign_id, contributer);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn lock_campaign(campaign_id: CampaignId) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().lock_campaign(campaign_id);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn submit_watch_finalize<Call>(
    call: Call,
) -> Result<ExtrinsicEvents<DatahighwayConfig>, subxt::Error>
where
    Call: TxPayload,
{
    let signer = signer();
    api()
        .tx()
        .sign_and_submit_then_watch(&call, &signer, Default::default())
        .await?
        .wait_for_finalized_success()
        .await
}

lazy_static! {
    static ref SIGNER: PairSigner = {
        let key_path = "signer.key";
        let phrase = std::fs::read_to_string(key_path).unwrap();
        let password = std::env::var("PASSWORD").ok().unwrap_or_default();
        let (pair, _seed) =
            Sr25519Pair::from_phrase(phrase.as_str(), Some(password.as_str())).unwrap();

        PairSigner::new(pair)
    };
    static ref DATAHIGHWAY_API: DatahighwayOnlineClient = {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            DatahighwayOnlineClient::from_url(NODE_CONNECT)
                .await
                .unwrap()
        })
    };
}

fn api() -> &'static DatahighwayOnlineClient {
    &DATAHIGHWAY_API
}

fn signer() -> PairSigner {
    SIGNER.to_owned()
}

type CreateCampaignParams = campaign_types::CreateCampaignParam<AccountId, BlockNumber>;
type UpdateCampaignParams = campaign_types::UpdateCampaignParam<AccountId, BlockNumber>;
type CampaignInfo = campaign_types::CampaignReward<AccountId, BlockNumber>;
type CampaignId = u32;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Contributer {
    pub who: AccountId,
    contributed: Balance,
    contributing: Balance,
}

impl Contributer {
    pub fn reward_amount(&self) -> Balance {
        // convert contributed amount to reward amount
        // FIXME:
        self.contributed.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Campaign {
    pub campaign_id: CampaignId,
    pub instant_percentage: (u32, u32),
    pub starts_from: BlockNumber,
    pub ends_at: BlockNumber,
    pub hoster: AccountId,
    pub contributers: Vec<Contributer>,
}

impl Campaign {
    pub async fn create(&self) -> Result<(), Box<dyn StdError>> {
        start_campaign(
            self.campaign_id,
            CreateCampaignParams {
                hoster: Some(self.hoster.clone()),
                instant_percentage: {
                    let (numenator, denomator) = self.instant_percentage;
                    campaign_types::SmallRational {
                        numenator,
                        denomator,
                    }
                },
                starts_from: Some(self.starts_from),
                end_target: self.ends_at,
            },
        )
        .await
    }

    pub async fn populate_contributer(&self) -> Result<(), Box<dyn StdError>> {
        for contributer in self.contributers.iter() {
            add_contributer(
                self.campaign_id,
                contributer.who.clone(),
                contributer.reward_amount(),
            )
            .await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputFile {
    campaign: Campaign,
    contributers_file: PathBuf,
}

impl InputFile {
    pub fn process(self) -> Result<Campaign, Box<dyn StdError>> {
        let Self {
            mut campaign,
            contributers_file,
        } = self;

        let contributer_file = File::open(contributers_file)?;
        let reader = BufReader::new(contributer_file);
        let contributers = serde_json::from_reader::<_, Vec<Contributer>>(reader)?;

        campaign.contributers = contributers;

        Ok(campaign)
    }
}
