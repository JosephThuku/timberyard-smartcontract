#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// Define the valid timber types and sizes as constants
const VALID_TIMBER_TYPES: &[&str] = &["cyprus", "pine", "oak", "cedar", "spruce"];
const VALID_TIMBER_SIZES: &[&str] = &[
    "2x4", "2x6", "2x8", "2x10", "3x2", "3x4", "4x2", "4x4", "4x6", "6x2", "6x4", "8x2", "8x4",
    "10x2", "10x4",
];

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Timber {
    id: u64,
    timber_type: String,
    timber_size: String,
    quantity: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Sales {
    id: u64,
    timber_id: u64,
    quantity: u64,
    price: u64,
    created_at: u64,
    updated_at: Option<u64>,
}

// Implement the Storable and BoundedStorable traits for the Timber struct
impl Storable for Timber {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Timber {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Implement the Storable and BoundedStorable traits for the Sales struct
impl Storable for Sales {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Sales {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Thread-local storage for the memory manager and storage structures
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static TIMBER_STORAGE: RefCell<StableBTreeMap<u64, Timber, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));

    static SALES_STORAGE: RefCell<StableBTreeMap<u64, Sales, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(2)))
    ));
}

// Structs to hold payloads for timber and sales
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TimberPayload {
    timber_type: String,
    timber_size: String,
    quantity: u64,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SalesPayload {
    timber_id: u64,
    quantity: u64,
    price: u64,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct SalesUpdatePayload {
    id: u64,
    quantity: u64,
    price: u64,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct TimberUpdatePayload {
    id: u64,
    timber_type: String,
    timber_size: String,
    quantity: u64,
}

// Function to generate a new unique ID
fn generate_unique_id() -> u64 {
    ID_COUNTER
        .with(|counter| {
            let mut counter = counter.borrow_mut();
            let id = *counter.get() + 1;
            counter.set(id).expect("Failed to increment ID counter");
            id
        })
}

// Function to get a timber by id
#[ic_cdk::query]
fn get_timber(id: u64) -> Result<Timber, String> {
    match _get_timber(&id) {
        Some(timber) => Ok(timber),
        None => Err(format!("Timber with id={} not found", id)),
    }
}

// Function to get a sales by id
#[ic_cdk::query]
fn get_sales(id: u64) -> Result<Sales, String> {
    match _get_sales(&id) {
        Some(sales) => Ok(sales),
        None => Err(format!("Sales with id={} not found", id)),
    }
}

// Function to add a timber with input validation
#[ic_cdk::update]
fn add_timber(timber: TimberPayload) -> Result<Timber, String> {
    // Validate input
    if !VALID_TIMBER_TYPES.contains(&timber.timber_type.as_str()) {
        return Err(format!(
            "Invalid timber type: {}. Valid types are: {:?}",
            timber.timber_type, VALID_TIMBER_TYPES
        ));
    }
    if !VALID_TIMBER_SIZES.contains(&timber.timber_size.as_str()) {
        return Err(format!(
            "Invalid timber size: {}. Valid sizes are: {:?}",
            timber.timber_size, VALID_TIMBER_SIZES
        ));
    }
    if timber.quantity == 0 {
        return Err("Quantity must be greater than zero".to_string());
    }

    let id = generate_unique_id();
    let timber = Timber {
        id,
        timber_type: timber.timber_type,
        timber_size: timber.timber_size,
        quantity: timber.quantity,
        created_at: time(),
        updated_at: None,
    };
    do_insert_timber(&timber);
    Ok(timber)
}

// Function to add a sales record with input validation
#[ic_cdk::update]
fn add_sales(sales: SalesPayload) -> Result<Sales, String> {
    // Validate input
    if sales.quantity == 0 {
        return Err("Quantity must be greater than zero".to_string());
    }
    if sales.price == 0 {
        return Err("Price must be greater than zero".to_string());
    }

    // Check if timber_id exists
    match _get_timber(&sales.timber_id) {
        Some(_) => (),
        None => return Err(format!("Timber with id={} not found", sales.timber_id)),
    }

    let id = generate_unique_id();
    let sales = Sales {
        id,
        timber_id: sales.timber_id,
        quantity: sales.quantity,
        price: sales.price,
        created_at: time(),
        updated_at: None,
    };
    do_insert_sales(&sales);
    Ok(sales)
}

// Function to update a timber with input validation
#[ic_cdk::update]
fn update_timber(id: u64, payload: TimberUpdatePayload) -> Result<Timber, String> {
    // Validate input
    if !VALID_TIMBER_TYPES.contains(&payload.timber_type.as_str()) {
        return Err(format!(
            "Invalid timber type: {}. Valid types are: {:?}",
            payload.timber_type, VALID_TIMBER_TYPES
        ));
    }
    if !VALID_TIMBER_SIZES.contains(&payload.timber_size.as_str()) {
        return Err(format!(
            "Invalid timber size: {}. Valid sizes are: {:?}",
            payload.timber_size, VALID_TIMBER_SIZES
        ));
    }
    if payload.quantity == 0 {
        return Err("Quantity must be greater than zero".to_string());
    }

    // Update timber
    match TIMBER_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut timber) => {
            timber.timber_type = payload.timber_type;
            timber.timber_size = payload.timber_size;
            timber.quantity = payload.quantity;
            timber.updated_at = Some(time());
            do_insert_timber(&timber);
            Ok(timber)
        }
        None => Err(format!(
            "Couldn't update timber with id={}. Timber not found",
            id
        )),
    }
}

// Function to update a sales record with input validation
#[ic_cdk::update]
fn update_sales(id: u64, payload: SalesUpdatePayload) -> Result<Sales, String> {
    // Validate input
    if payload.quantity == 0 {
        return Err("Quantity must be greater than zero".to_string());
    }
    if payload.price == 0 {
        return Err("Price must be greater than zero".to_string());
    }

    // Update sales
    match SALES_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut sales) => {
            sales.quantity = payload.quantity;
            sales.price = payload.price;
            sales.updated_at = Some(time());
            do_insert_sales(&sales);
            Ok(sales)
        }
        None => Err(format!(
            "Couldn't update sales with id={}. Sales not found",
            id
        )),
    }
}

// Function to delete a timber
#[ic_cdk::update]
fn delete_timber(id: u64) -> Result<Timber, String> {
    match TIMBER_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(timber) => {
            TIMBER_STORAGE.with(|service| service.borrow_mut().remove(&id));
            Ok(timber)
        }
        None => Err(format!(
            "Couldn't delete timber with id={}. Timber not found",
            id
        )),
    }
}

// Function to delete a sales record
#[ic_cdk::update]
fn delete_sales(id: u64) -> Result<Sales, String> {
    match SALES_STORAGE.with(|service| service.borrow().get(&id)) {
        Some(sales) => {
            SALES_STORAGE.with(|service| service.borrow_mut().remove(&id));
            Ok(sales)
        }
        None => Err(format!(
            "Couldn't delete sales with id={}. Sales not found",
            id
        )),
    }
}

// Helper method to perform insert operation for timber
fn do_insert_timber(timber: &Timber) {
    TIMBER_STORAGE.with(|service| service.borrow_mut().insert(timber.id, timber.clone()));
}

// Helper method to perform insert operation for sales
fn do_insert_sales(sales: &Sales) {
    SALES_STORAGE.with(|service| service.borrow_mut().insert(sales.id, sales.clone()));
}

// Helper methods for querying data

fn _get_timber(id: &u64) -> Option<Timber> {
    TIMBER_STORAGE.with(|service| service.borrow().get(id))
}

fn _get_sales(id: &u64) -> Option<Sales> {
    SALES_STORAGE.with(|service| service.borrow().get(id))
}

fn _get_timber_by_type(timber_type: &str) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_type == timber_type)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_timber_by_size(timber_size: &str) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_size == timber_size)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_sales_by_timber_id(timber_id: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.timber_id == *timber_id)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

fn _get_sales_by_price(price: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.price == *price)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

fn _get_sales_by_quantity(quantity: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.quantity == *quantity)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

fn _get_sales_by_id(id: &u64) -> Option<Sales> {
    SALES_STORAGE.with(|service| service.borrow().get(id))
}

fn _get_timber_by_id(id: &u64) -> Option<Timber> {
    TIMBER_STORAGE.with(|service| service.borrow().get(id))
}

fn _get_timber_by_quantity(quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.quantity == *quantity)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_timber_by_created_at(created_at: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.created_at == *created_at)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_sales_by_created_at(created_at: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.created_at == *created_at)
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

fn _get_timber_by_updated_at(updated_at: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.updated_at == Some(*updated_at))
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_sales_by_updated_at(updated_at: &u64) -> Vec<Sales> {
    SALES_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, sales)| sales.updated_at == Some(*updated_at))
                .map(|(_, sales)| sales.clone())
                .collect()
        })
}

fn _get_timber_by_type_and_size(timber_type: &str, timber_size: &str) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| {
                    timber.timber_type == timber_type && timber.timber_size == timber_size
                })
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_timber_by_type_and_quantity(timber_type: &str, quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_type == timber_type && timber.quantity == *quantity)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_timber_by_size_and_quantity(timber_size: &str, quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| timber.timber_size == timber_size && timber.quantity == *quantity)
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

fn _get_timber_by_type_and_size_and_quantity(timber_type: &str, timber_size: &str, quantity: &u64) -> Vec<Timber> {
    TIMBER_STORAGE
        .with(|service| {
            service
                .borrow()
                .iter()
                .filter(|(_, timber)| {
                    timber.timber_type == timber_type && timber.timber_size == timber_size && timber.quantity == *quantity
                })
                .map(|(_, timber)| timber.clone())
                .collect()
        })
}

// Export the candid interface
ic_cdk::export_candid!();
