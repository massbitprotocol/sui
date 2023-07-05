// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import {
	type KioskContents,
	type OriginByteKioskContents,
	getKioskIdFromDynamicFields,
	hasDisplayData,
	useGetKioskContents,
} from '@mysten/core';
import { getObjectDisplay, SuiObjectResponse, type SuiObjectData } from '@mysten/sui.js';

import cl from 'classnames';
import { NFTDisplayCard, type NFTDisplayCardProps } from '.';
import { useActiveAddress } from '../../hooks';
import { Text } from '../../shared/text';
import { NftImage, nftImageStyles } from './NftImage';

type KioskProps = {
	object: SuiObjectResponse;
} & Partial<NFTDisplayCardProps>;

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
	const items = (suiKiosk?.items ?? obKiosk?.items).sort((item) => (hasDisplayData(item) ? -1 : 1));

	return (
		<div className="relative hover:bg-transparent group transition-all flex justify-between h-36 w-36 rounded-xl">
			<div className="absolute z-0">
				{items.length &&
					items.slice(0, 3).map((item, idx) => {
						const display = getObjectDisplay(item)?.data;
						if (!display) return null;
						return (
							<div
								className={cl(
									'absolute transition-all ease-ease-out-cubic duration-250 rounded-xl',
									styles[idx],
								)}
							>
								<NftImage
									src={display.image_url}
									title={display.description}
									borderRadius={nftDisplayCardProps.borderRadius}
									size={nftDisplayCardProps.size}
									name="Kiosk"
									showLabel
								/>
							</div>
						);
					})}
			</div>
			<div className="right-1.5 bottom-1.5 flex items-center justify-center absolute h-6 w-6 bg-gray-100 text-white rounded-md">
				<Text variant="subtitle" weight="medium">
					{items?.length}
				</Text>
			</div>
		</div>
	);
}
