#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{ Randomness, Currency, ReservableCurrency, ExistenceRequirement };

	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_128;
	use sp_runtime::traits::{ AtLeast32BitUnsigned, Bounded, CheckedAdd }; 

	// type KittyIndex = u32; //kitty的数量就是u32这么大
    
	type BalanceOf<T> = 
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::type_value]
	pub fn GetDefaultValue<T: Config>() -> T::KittyIndex {
		0_u8.into()
	}
    //kitty的数据存储
	#[derive(Encode, Clone, Debug, PartialEq, Eq, Decode, TypeInfo, MaxEncodedLen)]
	pub struct Kitty(pub [u8; 16]);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		// 事件
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// 引入Randomness随机类型
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		/// 引入资产类型，以便支持质押
		/// 参考：substrate/frame/treasury/src/lib.rs中的定义
		type Currency: ReservableCurrency<Self::AccountId>;
		// 定义KittyIndex类型，要求实现执行的trait
		// Paramter 表示可以用于函数参数传递
		// AtLeast32Bit 表示转换为u32不会造成数据丢失
		// Default 表示有默认值
		// Copy 表示实现Copy方法
		// Bounded 表示包含上界和下界
		// 以后开发遇到在Runtime中定义无符号整型，可以直接复制套用
		type KittyIndex:Parameter + AtLeast32BitUnsigned + Default + Copy + Bounded + MaxEncodedLen;
        // 定义常量时，必须带上以下宏 变量定义在constract
		#[pallet::constant]
		// 获取Runtime中Kitties pallet定义的质押金额常量
		// 在创建Kitty前需要做质押，避免反复恶意创建
		type KittyPrice:Get<BalanceOf<Self>>;
        
		#[pallet::constant]
		type MaxKittyIndex:Get<u32>;

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn next_kitty_id)] // getter声明外部要查询存储时，可以调用next_kitty_id方法，方法名称可自定义
	/// 存储kitty最新的id，用作索引，也可以用作kitty数量总计(+1)
	pub type NextKittyId<T:Config> = StorageValue<_, T::KittyIndex, ValueQuery, GetDefaultValue<T>>; //ktiit的id从0开始
    // 所有kitties  用哈希map来存储，id => Kitty结构体
	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T:Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Kitty>;
    

	//kitty的所有者
	// 存储kitty对应的owner  用哈希map来存储，id => AccountId
	// 通过KittyIndex来查找Owner 
	#[pallet::storage]
	#[pallet::getter(fn kitty_owner)]
	pub type KittyOwner<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, T::AccountId>;
    
    // Kitty交易市场 存储正在销售的Kitty  KittyIndex => BalanceOf 即指定Kitty => 报价
    // 如果 Option<BalanceOf<T>> 为None, 意味着该Kitty不参与销售.
    #[pallet::storage]
	#[pallet::getter(fn kitties_list_for_sales)]
	pub type KittiesShop<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<BalanceOf<T>>, ValueQuery>;

	//全部的kitty id的数组
	#[pallet::storage]
	#[pallet::getter(fn all_kitties)]
	pub type AllKitties<T:Config> = StorageMap<
	_, 
	Blake2_128Concat,
	T::AccountId,
	BoundedVec<T::KittyIndex,T::MaxKittyIndex>,
	ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		KittyCreated(T::AccountId, T::KittyIndex, Kitty),
		KittyBred(T::AccountId, T::KittyIndex, Kitty),
		KittyTransferred(T::AccountId, T::AccountId, T::KittyIndex),
	}
	#[pallet::error]
	pub enum Error<T> {
		InvalidKittyId,
		KittyIdOverflow,
		NotOwner,
		/// 重复的kitty_id
		SameKittyId,
		  /// 买卖Kitty时，不能自己买自己
		NoBuySelf, 
		  /// 非卖品
		NotForSale,    
		  /// 没有足够的余额
		NotEnoughBalance, 
  
		OwnTooManyKitties,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// 0.9.25版本，在执行链方法时，为了保证事务性，必须在每个方法前加上宏`#[frame_support::transactional]`。
		// 0.9.25之后的版本，方法会默认加上事务属性。
		#[pallet::weight(10_000)]
		pub fn create(origin: OriginFor<T>) -> DispatchResultWithPostInfo  {
			// 获取当前操作者账户
			let who = ensure_signed(origin)?;
			//判断是否被占满
			// let kitty_id = Self::get_next_id().map_err(|_| Error::<T>::InvalidKittyId)?;

			let dna = Self::random_value(&who);
			// let kitty = Kitty(dna);
            Self::new_kitty_with_stake(&who, dna)?;
			// Kitties::<T>::insert(kitty_id, &kitty);
			// KittyOwner::<T>::insert(kitty_id, &who);
			// NextKittyId::<T>::set(kitty_id + 1);

			// Self::deposit_event(Event::KittyCreated(who, kitty_id, kitty));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn breed(
			origin: OriginFor<T>,
			kitty_id_1: T::KittyIndex,
			kitty_id_2: T::KittyIndex,
		) -> DispatchResultWithPostInfo  {
			let who = ensure_signed(origin)?;

			//check kitty id
			ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameKittyId);
			let kitty_1 = Self::get_kitty(kitty_id_1).map_err(|_| Error::<T>::InvalidKittyId)?;
			let kitty_2 = Self::get_kitty(kitty_id_2).map_err(|_| Error::<T>::InvalidKittyId)?;
			//get next id
			// let kitty_id = Self::get_next_id().map_err(|_| Error::<T>::InvalidKittyId)?;
			//seleector for breeding
			let seleector = Self::random_value(&who);
			let mut new_dna: [u8; 16] = [0u8; 16];
			for i in 0..kitty_1.0.len() {
				new_dna[i] = (kitty_1.0[i] & seleector[i]) | (kitty_2.0[i] & !seleector[i]);
			}

			// let new_kitty = Kitty(data);
			// <Kitties<T>>::insert(kitty_id, &new_kitty);
			// KittyOwner::<T>::insert(kitty_id, &who);
			// NextKittyId::<T>::set(kitty_id + 1);

			// Self::deposit_event(Event::KittyCreated(who, kitty_id, new_kitty));
            Self::new_kitty_with_stake(&who, new_dna)?;
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		 /// 转让Kitties
		pub fn transfer(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			new_owner: T::AccountId,
		) -> DispatchResult {
			 // 获取当前操作者账户
			 let sender = ensure_signed(origin)?;

			// 检查kitty_id是否有效
			// map_err ? 可以理解成三目运算符，满足条件就返回值，不满足条件就返回错误信息
			Self::get_kitty(kitty_id).map_err(|_| Error::<T>::InvalidKittyId)?;

            // 检查是否为kitty的owner
			// 只有条件为true时，才不会报后面的Error，ensure!()相当于solidity中的require()
			ensure!(Self::kitty_owner(kitty_id) == Some(sender.clone()), Error::<T>::NotOwner);
			// 获取需要质押的金额
            let stake_amount = T::KittyPrice::get();

			// 新的Owner账户进行质押
            T::Currency::reserve(&new_owner, stake_amount).map_err(|_| Error::<T>::NotEnoughBalance)?;

			// 旧的Owner账户解除质押
            T::Currency::unreserve(&sender, stake_amount);

			// 保存kitty的新owner 更新也使用insert，即重新插入一条新记录覆盖原来的老数据
			<KittyOwner<T>>::insert(&kitty_id, &new_owner);

			AllKitties::<T>::try_mutate(&sender, |ref mut kitties| {
				let index = kitties.iter().position(|&r| r == kitty_id).unwrap();
				kitties.remove(index);
				Ok::<(), DispatchError>(())
			})?;
			AllKitties::<T>::try_mutate(&new_owner, |ref mut kitties| {
				kitties.try_push(kitty_id).map_err(|_| Error::<T>::OwnTooManyKitties)?;
				Ok::<(), DispatchError>(())
			})?;
			 // 触发事件
			 Self::deposit_event(Event::KittyTransferred(sender, new_owner, kitty_id));
			 // 返回OK
			 Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		//get a random 256
		fn random_value(sender: &T::AccountId) -> [u8; 16] {
			let payload = (
				T::Randomness::random_seed(),
				&sender,
				<frame_system::Pallet<T>>::extrinsic_index(),
			);

			payload.using_encoded(blake2_128)
		}
		//get next id
		fn get_next_id() -> Result<T::KittyIndex, ()> {
			let kitty_id = Self::next_kitty_id();
			// match有点类似js/go语言中的switch
			// 理解学习模式匹配：https://course.rs/basic/match-pattern/all-patterns.html
			// _ 相当于 default:
			match kitty_id {
				_ if kitty_id >= T::KittyIndex::max_value() => Err(()),
				val => Ok(val),
			}
		}
		//get kitty via id
		fn get_kitty(kitty_id: T::KittyIndex) -> Result<Kitty, ()> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty),
				None => Err(()),
			}
		}
		// 质押并创建Kitty
        // 用于优化create()和breed()
		// TODO  可考虑加入Kitty的繁殖代数
        fn new_kitty_with_stake(sender: &T::AccountId, dna: [u8; 16]) -> DispatchResultWithPostInfo {

			// 获取需要质押的金额
            let stake_amount = T::KittyPrice::get();

			// 质押指定数量的资产，如果资产质押失败则报错
			T::Currency::reserve(&sender, stake_amount)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;
			// 或：
			// ensure!(T::Currency::can_reserve(&sender, stake_amount), Error::<T>::NotEnoughBalance);

            // 获取一个最新的kitty_id，如果返回出错，则提示无效的ID
			// map_err ? 可以理解成三目运算符，满足条件就返回值，不满足条件就返回错误信息
			let kitty_id = Self::get_next_id()
				.map_err(|_| Error::<T>::InvalidKittyId)?;

			let kitty = Kitty(dna);

			// 保存数据
			Kitties::<T>::insert(kitty_id, &kitty); // 保存kitty信息
			KittyOwner::<T>::insert(kitty_id, &sender); // 保存kitty的owner
			let next_kitty_id = kitty_id
				.checked_add(&(T::KittyIndex::from(1_u8))) // 检查溢出，参考：https://paritytech.github.io/substrate/master/sp_runtime/traits/index.html
				.ok_or(Error::<T>::KittyIdOverflow)
				.unwrap();
			NextKittyId::<T>::set(next_kitty_id); // kitty_id+1

			AllKitties::<T>::try_mutate(&sender, |ref mut kitties| {
				kitties.try_push(kitty_id).map_err(|_| Error::<T>::OwnTooManyKitties)?;
				Ok::<(), DispatchError>(())
			})?;

			// 触发事件
			Self::deposit_event(Event::KittyCreated(sender.clone(), kitty_id, kitty));

			// 返回OK
			Ok(().into())
        }
	}
}

