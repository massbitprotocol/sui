// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import {
	type KioskContents,
	type OriginByteKioskContents,
	getKioskIdFromDynamicFields,
	hasDisplayData,
	useGetKioskContents,
} from '@mysten/core';
import { type SuiObjectData } from '@mysten/sui.js';

import { NFTDisplayCard, type NFTDisplayCardProps } from '.';
import { useActiveAddress } from '../../hooks';
import { Text } from '../../shared/text';

interface KioskProps extends NFTDisplayCardProps {
	object: SuiObjectData;
}

const styles: Record<number, string> = {
	0: 'group-hover:scale-95 shadow-md z-20 -top-[0px]',
	1: 'scale-[0.975] group-hover:-rotate-6 group-hover:-translate-x-6 group-hover:-translate-y-3 z-10 -top-[6px]',
	2: 'scale-95 group-hover:rotate-6 group-hover:translate-x-6 group-hover:-translate-y-3 z-0 -top-[12px]',
};

export function Kiosk({ object, ...nftDisplayCardProps }: KioskProps) {
	const address = useActiveAddress();
	const { data: kioskData } = useGetKioskContents(address);
	const kioskId = getKioskIdFromDynamicFields(object);
	const suiKiosk = kioskData?.kiosks.sui.get(kioskId) as KioskContents;
	const obKiosk = kioskData?.kiosks.originByte.get(kioskId) as OriginByteKioskContents;
	const items = (suiKiosk?.items ?? obKiosk?.items)
		.sort((item) => (hasDisplayData(item) ? -1 : 1))
		.slice(0, 3);

	return (
		<div className="relative ease-ease-out-cubic origin-bottom duration-250 hover:bg-transparent group transition-all h-full w-full bg-black flex flex-col justify-between">
			<div className="absolute z-0">
				{items.length &&
					items.map((item, idx) => {
						return (
							<div className={`absolute transition-all ease-ease-out-cubic ${styles[idx] ?? ''}`}>
								<NFTDisplayCard
									{...nftDisplayCardProps}
									animateHover={false}
									objectId={item.data?.objectId!}
								/>
							</div>
						);
					})}
			</div>
			<div className="right-5 bottom-5.5 flex items-center justify-center absolute h-6 w-6 bg-gray-100 text-white rounded-md">
				<Text variant="subtitle" weight="medium">
					{items?.length}
				</Text>
			</div>

			<div className="flex-1 mt-2 text-steel-dark truncate overflow-hidden max-w-full group-hover:text-black duration-200 ease-ease-in-out-cubic">
				Kiosk
			</div>
		</div>
	);
}
