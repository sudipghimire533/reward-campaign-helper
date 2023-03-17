const NODE_CONNECT: &'static str = "ws://127.0.0.1:9988";

#[subxt::subxt(runtime_metadata_path = "chain-metadata.scale")]
pub mod datahighway {}

use clap::builder::StringValueParser;
use datahighway::runtime_types::pallet_reward_campaign::types as campaign_types;
use lazy_static::lazy_static;
use serde::de::value::StringDeserializer;
use serde::{Deserialize, Deserializer, Serialize};
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
use subxt::tx::{TxInBlock, TxPayload};
use subxt::utils::AccountId32;
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

    let api = DatahighwayOnlineClient::from_url(NODE_CONNECT)
        .await
        .unwrap();

    // read the input file
    let input_file_name = std::env::args()
        .skip(1)
        .next()
        .unwrap_or("reward-campaign.json".to_string());
    let input_file = File::open(input_file_name).unwrap();
    println!("Input file: {:?}", input_file);
    let input_file_reader = BufReader::new(input_file);
    let input = serde_json::from_reader::<_, InputFile>(input_file_reader).unwrap();
    let campaign = input.process().unwrap();

    // this is the campaign_id
    let campaign_id = campaign.campaign_id;

    campaign.create(&api).await.unwrap();
    println!("Campaign started...");

    campaign.populate_contributer(&api).await.unwrap();
    println!("All contributer processed..");

    Ok(())
}

async fn start_campaign(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
    campaign_info: CreateCampaignParams,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .start_new_campaign(campaign_id, campaign_info);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn update_campaign(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
    new_campaign_info: UpdateCampaignParams,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .update_campaign(campaign_id, new_campaign_info);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn discard_campaign(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().discard_campaign(campaign_id);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn wipe_campaign(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().wipe_campaign(campaign_id);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn add_contributer(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
    contributer: AccountId,
    amount: Balance,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .add_contributer(campaign_id, contributer, amount);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn remove_contributer(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
    contributer: AccountId,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .remove_contributer(campaign_id, contributer);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn lock_campaign(
    api: &DatahighwayOnlineClient,
    campaign_id: CampaignId,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().lock_campaign(campaign_id);
    submit_and_watch(api, call).await?;

    Ok(())
}

async fn submit_and_watch<Call>(
    api: &DatahighwayOnlineClient,
    call: Call,
) -> Result<TxInBlock<DatahighwayConfig, DatahighwayOnlineClient>, subxt::Error>
where
    Call: TxPayload,
{
    let signer = signer();
    api.tx()
        .sign_and_submit_then_watch(&call, &signer, Default::default())
        .await?
        .wait_for_in_block()
        .await
}

lazy_static! {
    static ref SIGNER: PairSigner = {
        let key_path = std::env::var("SIGNER_KEY").unwrap_or("signer.key".to_string());
        println!("Key path: {key_path}");
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
    #[serde(deserialize_with = "string_to_balance")]
    contributed: Balance,
}

fn string_to_balance<'de, D>(input: D) -> Result<Balance, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let quoted: String = Deserialize::deserialize(input)?;
    Ok(quoted.parse::<Balance>().unwrap())
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
    #[serde(rename = "campaignId")]
    pub campaign_id: CampaignId,
    #[serde(rename = "instantPercentage")]
    pub instant_percentage: (u32, u32),
    #[serde(rename = "startsFrom")]
    pub starts_from: BlockNumber,
    #[serde(rename = "endsAt")]
    pub ends_at: BlockNumber,
    #[serde(rename = "hoster")]
    pub hoster: AccountId,
    #[serde(skip)]
    pub contributers: Vec<Contributer>,
}

impl Campaign {
    pub async fn create(&self, api: &DatahighwayOnlineClient) -> Result<(), Box<dyn StdError>> {
        start_campaign(
            api,
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

    pub async fn populate_contributer(
        &self,
        api: &DatahighwayOnlineClient,
    ) -> Result<(), Box<dyn StdError>> {
        for contributer in self.contributers.iter() {
            if let Err(err) = add_contributer(
                api,
                self.campaign_id,
                contributer.who.clone(),
                contributer.reward_amount(),
            )
            .await
            {
                eprintln!(
                    "Error while adding contributer: {}. Error: {err:?}",
                    contributer.who
                );
                eprintln!("Skipping and moving to next..");
            }

            println!("Contributer {} added to campaign...", contributer.who);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputFile {
    campaign: Campaign,
    #[serde(rename = "contributers")]
    contributers_file: PathBuf,
}

impl InputFile {
    pub fn process(self) -> Result<Campaign, Box<dyn StdError>> {
        let Self {
            mut campaign,
            contributers_file,
        } = self;

        println!("Contributer file: {:?}", contributers_file);
        let contributer_file = File::open(contributers_file)?;
        let reader = BufReader::new(contributer_file);
        let contributers = serde_json::from_reader::<_, Vec<Contributer>>(reader)?;

        campaign.contributers = contributers;

        Ok(campaign)
    }
}
